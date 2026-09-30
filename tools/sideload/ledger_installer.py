#!/usr/bin/env python3
"""Install (sideload) this Ledger app on a physical device over USB.

Meant to be frozen into a single executable with the bundle produced by `make_bundle.py` embedded, so
that an end user only has to plug the device and run it. The APDU scripts are replayed through the
same secure channel as `python -m ledgerblue.runScript --scp`, which is what `cargo ledger build --load`
uses.
"""

import argparse
import contextlib
import hashlib
import io
import json
import sys
from dataclasses import dataclass
from pathlib import Path

from ledgerblue.comm import get_possible_error_cause, getDongle
from ledgerblue.commException import CommException
from ledgerblue.deployed import getDeployedSecretV2
from ledgerblue.ecWrapper import PrivateKey
from ledgerblue.hexLoader import HexLoader

MANIFEST_NAME = "manifest.json"
MANIFEST_SCHEMA = 1

# Dashboard "get device info" command: the response starts with the 4-byte target ID.
GET_DEVICE_INFO_APDU = bytes([0xE0, 0x01, 0x00, 0x00, 0x00])
TARGET_ID_LEN = 4
APDU_HEADER_LEN = 5
SECURE_CHANNEL_CLA = 0xE0

# Display names only; the bundle manifest is the authority on which targets can be installed.
DEVICE_NAMES = {
    0x33000004: "Nano X",
    0x33100004: "Nano S Plus",
    0x33200004: "Stax",
    0x33300004: "Flex",
    0x33400004: "Apex P",
}
NANO_X_TARGET_ID = 0x33000004

SW_DENIED = 0x6985
SW_LOCKED = 0x5515
SW_NOT_ON_DASHBOARD = (0x6D00, 0x6E00)
SW_OS_INCOMPATIBLE = 0x511F
SW_NO_SIDELOAD = 0x5120
SW_NOT_ENOUGH_SPACE = (0x6A84, 0x6A85)

LINUX_UDEV_HINT = (
    "On Linux, the device is only visible once Ledger's udev rules are installed:\n"
    "    wget -q -O - https://raw.githubusercontent.com/LedgerHQ/udev-rules/master/add_udev_rules.sh | sudo bash\n"
    "then unplug and replug the device."
)


class InstallerError(Exception):
    """An error with a message meant for the end user."""


@dataclass(frozen=True)
class AppEntry:
    device: str
    target_id: int
    api_level: str
    sdk_version: str
    app_hash: str
    apdu_path: Path
    apdu_sha256: str


@dataclass(frozen=True)
class Source:
    ref: str
    commit: str
    official: bool

    def describe(self) -> str:
        return f"{self.ref} @ {self.commit[:12]}" if self.commit else self.ref


@dataclass(frozen=True)
class Bundle:
    app_name: str
    app_version: str
    source: Source
    apps: list[AppEntry]

    def for_target(self, target_id: int) -> AppEntry | None:
        return next((app for app in self.apps if app.target_id == target_id), None)


def default_bundle_dir() -> Path:
    # PyInstaller unpacks embedded data files under sys._MEIPASS.
    base = Path(getattr(sys, "_MEIPASS", Path(__file__).resolve().parent))
    return base / "bundle"


def load_bundle(bundle_dir: Path) -> Bundle:
    manifest_path = bundle_dir / MANIFEST_NAME
    if not manifest_path.is_file():
        raise InstallerError(f"No app bundle found ({manifest_path} is missing).")
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("schema") != MANIFEST_SCHEMA:
        raise InstallerError(f"Unsupported bundle format (schema {manifest.get('schema')!r}).")
    apps = [
        AppEntry(
            device=entry["device"],
            target_id=int(entry["target_id"], 16),
            api_level=entry["api_level"],
            sdk_version=entry["sdk_version"],
            app_hash=entry["app_hash"],
            apdu_path=bundle_dir / entry["apdu_file"],
            apdu_sha256=entry["apdu_sha256"],
        )
        for entry in manifest["apps"]
    ]
    source = Source(
        ref=manifest["source"]["ref"],
        commit=manifest["source"]["commit"],
        official=manifest["source"]["official"] is True,
    )
    return Bundle(app_name=manifest["app_name"], app_version=manifest["app_version"], source=source, apps=apps)


def read_verified_apdus(app: AppEntry) -> list[bytes]:
    """Read the install script, refusing it if it does not match the manifest checksum."""
    content = app.apdu_path.read_bytes()
    if hashlib.sha256(content).hexdigest() != app.apdu_sha256:
        raise InstallerError(f"The app file {app.apdu_path.name} is corrupted (checksum mismatch). Download the installer again.")
    apdus = [bytes.fromhex(line) for line in content.decode("ascii").splitlines() if line.strip()]
    if not apdus or any(len(apdu) < APDU_HEADER_LEN for apdu in apdus):
        raise InstallerError(f"The app file {app.apdu_path.name} is malformed.")
    return apdus


def device_name(target_id: int) -> str:
    return DEVICE_NAMES.get(target_id, f"unknown device (target ID {target_id:#010x})")


def explain_status(sw: int) -> str:
    if sw == SW_DENIED:
        return "The operation was rejected on the device."
    if sw == SW_LOCKED:
        return "The device is locked. Unlock it with your PIN and try again."
    if sw in SW_NOT_ON_DASHBOARD:
        return "An app is open on the device. Quit it to go back to the dashboard and try again."
    if sw == SW_OS_INCOMPATIBLE:
        return (
            "This build of the app does not match your device's OS version. Update your device with Ledger Live, "
            "then use the latest installer."
        )
    if sw == SW_NO_SIDELOAD:
        return "This device model does not allow installing apps outside Ledger Live."
    if sw in SW_NOT_ENOUGH_SPACE:
        return "Not enough space on the device. Uninstall some apps with Ledger Live and try again."
    return f"The device returned error {sw:#06x} ({get_possible_error_cause(sw)})."


def connect(verbose: bool):
    try:
        return getDongle(verbose)
    except CommException as e:
        hint = (
            "No Ledger device found. Check that it is plugged in over USB and unlocked, "
            "and that Ledger Live (or any other wallet app) is closed."
        )
        if sys.platform.startswith("linux"):
            hint += "\n" + LINUX_UDEV_HINT
        raise InstallerError(hint) from e


def get_target_id(dongle) -> int:
    response = dongle.exchange(bytearray(GET_DEVICE_INFO_APDU))
    if len(response) < TARGET_ID_LEN:
        raise InstallerError("Unexpected answer from the device. Make sure it is on the dashboard.")
    return int.from_bytes(response[:TARGET_ID_LEN], "big")


def open_secure_channel(dongle, target_id: int, verbose: bool) -> HexLoader:
    """Open the secure channel; the device asks the user to allow the (unofficial) manager."""
    root_private_key = PrivateKey()
    # ledgerblue prints key material details that are noise for end users.
    output = contextlib.nullcontext() if verbose else contextlib.redirect_stdout(io.StringIO())
    with output:
        secret = getDeployedSecretV2(dongle, bytearray.fromhex(root_private_key.serialize()), target_id)
    return HexLoader(dongle, SECURE_CHANNEL_CLA, True, secret)


def run_script(dongle, loader: HexLoader, apdus: list[bytes]) -> None:
    for index, apdu in enumerate(apdus, start=1):
        payload = loader.scpWrap(apdu[APDU_HEADER_LEN:])
        dongle.exchange(bytearray(apdu[:4]) + bytearray([len(payload)]) + bytearray(payload))
        print(f"\r  Transferring... {index * 100 // len(apdus):3d}%", end="", flush=True)
    print()


def confirm(question: str, assume_yes: bool) -> bool:
    if assume_yes:
        return True
    return input(f"{question} [y/N] ").strip().lower() in ("y", "yes")


def print_warning(bundle: Bundle) -> None:
    print(
        f"\n=== {bundle.app_name} {bundle.app_version} installer ===\n\n"
        "This tool installs an app that is NOT distributed by Ledger Live. Your device will warn you that the\n"
        "manager is not genuine: this is expected. Only continue if you trust where you got this installer from.\n"
        "Your recovery phrase is never needed for this: never type it anywhere.\n"
    )
    print(f"Built from: {bundle.source.describe()}")
    if not bundle.source.official:
        print(
            "\n!!! DEVELOPMENT BUILD: this is not a released version. It may contain unfinished, untested or\n"
            "!!! unreviewed code. Install it on a test device only, never on a device holding real funds.\n"
        )


def install(bundle: Bundle, args: argparse.Namespace) -> None:
    print("\nPlug in your Ledger device, unlock it and stay on the dashboard (no app open).")
    print("Close Ledger Live and any other wallet software.")
    if not args.yes:
        input("Press Enter when ready...")

    dongle = connect(args.verbose)
    try:
        target_id = get_target_id(dongle)
        print(f"Detected device: {device_name(target_id)}")
        if target_id == NANO_X_TARGET_ID:
            raise InstallerError("Nano X does not allow installing apps outside Ledger Live.")
        app = bundle.for_target(target_id)
        if app is None:
            supported = ", ".join(device_name(a.target_id) for a in bundle.apps)
            raise InstallerError(f"This installer has no build for this device. Supported devices: {supported}.")

        if args.uninstall:
            print("\nOn your device, allow the manager, then confirm the removal.")
            loader = open_secure_channel(dongle, target_id, args.verbose)
            loader.deleteApp(bundle.app_name.encode())
            print(f"\nDone: {bundle.app_name} was removed from your device.")
            return

        apdus = read_verified_apdus(app)
        print(f"\nApp identifier (shown on the device during installation):\n  {app.app_hash}")
        print("\nOn your device, allow the manager, then confirm the installation if asked.")
        loader = open_secure_channel(dongle, target_id, args.verbose)
        run_script(dongle, loader, apdus)
        print(f"\nDone: {bundle.app_name} {bundle.app_version} is installed on your device.")
    except CommException as e:
        raise InstallerError(explain_status(e.sw)) from e
    finally:
        dongle.close()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--bundle", type=Path, default=default_bundle_dir(), help="Bundle directory from make_bundle.py")
    parser.add_argument("--uninstall", action="store_true", help="Remove the app from the device instead of installing it")
    parser.add_argument("--list", action="store_true", help="Show the bundle content and exit")
    parser.add_argument("-y", "--yes", action="store_true", help="Do not ask for confirmation on the computer")
    parser.add_argument("-v", "--verbose", action="store_true", help="Log APDU exchanges")
    return parser.parse_args()


def is_double_clicked() -> bool:
    # A frozen executable launched from a file explorer gets no arguments; keep its console open at the end.
    return getattr(sys, "frozen", False) and len(sys.argv) == 1


def run(args: argparse.Namespace) -> int:
    bundle = load_bundle(args.bundle)
    if args.list:
        kind = "release" if bundle.source.official else "development build"
        print(f"{bundle.app_name} {bundle.app_version} ({kind}, built from {bundle.source.describe()})")
        for app in bundle.apps:
            print(f"  {device_name(app.target_id):<12} api_level={app.api_level} sdk={app.sdk_version} hash={app.app_hash}")
        return 0

    print_warning(bundle)
    action = "remove" if args.uninstall else "install"
    if not confirm(f"Do you want to {action} {bundle.app_name} {bundle.app_version}?", args.yes):
        print("Cancelled.")
        return 1
    install(bundle, args)
    return 0


def main() -> int:
    args = parse_args()
    try:
        status = run(args)
    except InstallerError as e:
        print(f"\nError: {e}", file=sys.stderr)
        status = 1
    except KeyboardInterrupt:
        print("\nCancelled.", file=sys.stderr)
        status = 1
    if is_double_clicked():
        with contextlib.suppress(EOFError):
            input("\nPress Enter to close this window.")
    return status


if __name__ == "__main__":
    sys.exit(main())

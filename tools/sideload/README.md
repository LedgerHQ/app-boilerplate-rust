# Sideload installer

A standalone executable that installs this app on a Ledger device over USB. End users don't need git,
Docker, Rust or Python: the release binaries are embedded in the executable.

## For end users

Supported devices: **Nano S Plus, Stax, Flex, Apex P**. Nano X does not allow installing apps outside
Ledger Live.

1. Open the project's GitHub **Releases** page, pick the version to install (see
   [Official releases and development builds](#official-releases-and-development-builds)) and download the
   installer for your computer:

   | Computer | File |
   | :--- | :--- |
   | Windows | `…-installer-windows-x86_64.exe` |
   | macOS (Apple Silicon: M1, M2…) | `…-installer-macos-arm64` |
   | macOS (Intel) | `…-installer-macos-x86_64` |
   | Linux | `…-installer-linux-x86_64` |

2. Plug in your Ledger device, unlock it and stay on the dashboard (no app open). **Close Ledger Live.**
3. Run the installer and follow the instructions:
   - **Windows**: double-click it. If SmartScreen warns you, click *More info* → *Run anyway*.
   - **macOS**: in Terminal, run `chmod +x ~/Downloads/<file> && xattr -d com.apple.quarantine ~/Downloads/<file> && ~/Downloads/<file>`.
   - **Linux**: install Ledger's udev rules once (`wget -q -O - https://raw.githubusercontent.com/LedgerHQ/udev-rules/master/add_udev_rules.sh | sudo bash`),
     then run `chmod +x <file> && ./<file>`.
4. On the device, allow the manager (the device warns that it is not genuine, which is expected for an app
   installed outside Ledger Live), then confirm the installation. The identifier on the device should match
   the one printed by the installer.

To remove the app, run the installer with `--uninstall`, or uninstall it from Ledger Live.

> [!WARNING]
> Only use installers downloaded from this project's official GitHub releases. Installing an app from
> anywhere else can put your funds at risk. The installer never asks for your recovery phrase: never type
> it on a computer.

### Official releases and development builds

| Release name | Built from | Replaced |
| :--- | :--- | :--- |
| `1.2.3` (version number) | An official release tag | Never |
| `Development build: branch main` (tag `build-main`) | A branch, on a maintainer's request | On every new build of that branch |

Development builds are marked as *pre-release*, and the installer shows a warning before installing them.
They may contain unfinished or unreviewed code: only install them on a test device. They are deleted
when their branch is deleted.

### Checking the download (optional)

Each release lists the installers' checksums in `SHA256SUMS` and has a GitHub build provenance
attestation, which proves that the file was built by this repository's CI:

```bash
gh attestation verify <file> --repo <owner>/<repo>
```

## For developers

| File | Role |
| :--- | :--- |
| `make_bundle.py` | Packages `cargo ledger build` outputs (`<device>/release/<elf>` + `.apdu` + `.sha256`) into a bundle: one `<device>.apdu` per device and a `manifest.json` (target ID, API level, app hash, checksum). |
| `ledger_installer.py` | Reads the device's target ID, picks the matching script, checks its checksum and replays it through the ledgerblue secure channel, like `cargo ledger build --load`. |
| `requirements.txt` | Pinned `ledgerblue` (runtime) and `pyinstaller` (build). |

[`sideload_installer.yml`](../../.github/workflows/sideload_installer.yml) builds the bundle and the four
executables, then publishes them with a `SHA256SUMS` file and build provenance attestations:

| Trigger | Published to |
| :--- | :--- |
| Release tag (called by [`release.yml`](../../.github/workflows/release.yml) with the release binaries) | That GitHub release |
| Manual run (*Actions* → *Build the sideload installer* → *Run workflow*, pick any branch) | Pre-release `build-<branch>` |

Nothing is built automatically on pushes or pull requests.

Characters outside `A-Za-z0-9._-` in branch names become `-` in tag names, so two branches that differ only
by those characters share the same pre-release.
[`sideload_cleanup.yml`](../../.github/workflows/sideload_cleanup.yml) deletes a pre-release and its tag
when its branch is deleted.

### Running it locally

Build the app first (`cargo ledger build <device>`, see the main README), then outside Docker:

```bash
pip install -r tools/sideload/requirements.txt
python3 tools/sideload/make_bundle.py target build/sideload/bundle   # --source-ref/--commit/--official describe the build
python3 tools/sideload/ledger_installer.py --bundle build/sideload/bundle      # add --verbose to log APDUs

# Optional: freeze into a single executable in build/sideload/dist/ (use ';' instead of ':' on Windows)
pyinstaller --onefile --name installer --add-data "$PWD/build/sideload/bundle:bundle" \
  --distpath build/sideload/dist --workpath build/sideload/work --specpath build/sideload \
  tools/sideload/ledger_installer.py
```

### Limitations

- The executables are not code-signed, so Windows SmartScreen and macOS Gatekeeper warn users before
  running them. Signing them needs an Apple Developer ID and a Windows code-signing certificate.
- An installer only works with the device OS versions its SDK targets. If the device reports an
  incompatible OS, users must update it with Ledger Live and use a newer release.

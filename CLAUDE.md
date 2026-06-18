# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Generic Ledger embedded-app rules (cross-language constraints, Rust/C/test/review conventions)
are imported from the `ledger-app-ai-instructions` submodule. This file documents only what is
**specific to this app**.

@ledger-app-ai-instructions/CLAUDE.md

## What this repo is

Boilerplate Ledger hardware-wallet application written in Rust, targeting Nano X, Nano S+, Stax, Flex, and Apex P. It is a `#![no_std]` / `#![no_main]` embedded binary built against `ledger_device_sdk`, and is intended to be forked as the starting point for new coin apps. Nano S is **not** supported.

Supported devices and the test directory layout are declared in `ledger_app.toml` — read this file rather than hard-coding device names or test paths. Use `README.md`, `ledger_app.toml`, and `.cargo/config.toml` as the sources of truth for build/test commands.

## Build, lint, test

All builds run inside the `ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest` Docker image (or via the Ledger VS Code extension). `cargo build` on the host will not work — there is no host target.

```bash
# Build for a specific device (output: target/<device>/release/app-boilerplate-rust)
cargo ledger build nanox          # one of: nanox | nanosplus | stax | flex | apex_p

# Build with debug logging enabled
cargo ledger build nanox -- --features debug -Zunstable-options

# Lint (CI runs cargo fmt over ./src)
cargo fmt --check

# Functional tests — must be run inside the dev-tools Docker container
pip install -r tests/standalone/requirements.txt
pytest tests/standalone/ --tb=short -v --device {nanosp|nanox|stax|flex|apex_p}

# Single test
pytest tests/standalone/test_sign_cmd.py -v --device nanox -k <test_name>

# Swap tests need Exchange + Ethereum binaries built first — see tests/swap/README.md
pytest tests/swap/ -v --device stax
```

Notes:
- `.cargo/config.toml` sets `apex_p` as the **default cargo target**; commands without an explicit `--target` or `cargo ledger build <device>` will build for Apex P.
- Toolchain is pinned in `rust-toolchain.toml` (currently `nightly-2025-12-05`); `build-std` is required for `core`/`alloc`.
- Heap is 8192 bytes by default. Override via `HEAP_SIZE` env var; allowed values: 2048, 4096, 8192, 16384, 24576.
- Tests use Ragger's snapshot navigation. **Never delete `snapshots/` or `snapshots-tmp/` manually.** Regenerate snapshots with `pytest ... --golden_run` only when a screen change is intentional.
- Speculos device name mapping: `ledger_app.toml` lists `nanos+` but pytest expects `--device nanosp`.

## Architecture

### Execution modes

`sample_main` (`src/main.rs`) dispatches based on whether the app was launched from the dashboard or via `os_lib_call` from the Exchange app:

- **Normal mode** (`arg0 == 0`): `normal_main(None)` — shows the home screen, runs the APDU loop, drives the review UI for signing.
- **Swap mode** (`arg0 != 0`): `swap::swap_main(arg0)` dispatches to `check_address` / `get_printable_amount` / `sign_transaction`. Signing reuses `normal_main(Some(&params))` with the UI **bypassed** — Exchange has already validated everything.

### APDU dispatch

1. `Comm` receives an APDU with `CLA=0xe0`; wrong-CLA APDUs are rejected automatically by the SDK (`comm.set_expected_cla(0xe0)`).
2. `Instruction::try_from(ApduHeader)` in `src/main.rs` parses `(INS, P1, P2)` into a strongly-typed enum. Invalid P1/P2 → `AppSW::WrongP1P2`; unknown INS → `AppSW::InsNotSupported`. **All APDU parameter validation lives here, not in the handlers.**
3. `handle_apdu` routes to a handler in `src/handlers/`. Handlers return `Result<CommandResponse, AppSW>`.
4. `AppSW` (status word enum in `src/main.rs`) is the single source of truth for error codes returned over the wire (e.g. `0x6985` = Deny, `0x9000` = Ok, app-specific tx/addr codes are `0xB001..0xB00A`, and swap failures are `0xC000`).
5. After each command, `show_status_and_home_if_needed` decides whether to display an NBGL success/failure screen and return to home — this **must** be called for `GetPubkey { display: true }` and completed `SignTx` flows in normal mode, and **must not** display anything in swap mode.

### Multi-chunk transactions

`SignTx` uses chunked transmission tracked in `TxContext` (`src/handlers/sign_tx.rs`):

- `P1=0, P2=0x80` — chunk 0 carries the BIP32 path only and resets the context.
- `P1=1..=3, P2=0x80` — intermediate chunks appended to `raw_tx`.
- `P1=1..=3, P2=0x00` — final chunk; triggers parse (`serde_json_core::from_slice` into `Tx`), UI review, and signing.
- `MAX_TRANSACTION_LEN = 510` is enforced on the accumulator; exceeding it returns `AppSW::TxWrongLength`.

Hash = Keccak256 of `raw_tx`; signature = secp256k1 deterministic, returned as `[siglen][DER signature][parity]`.

### Swap memory constraint (critical)

`src/swap.rs` runs **before** the SDK calls `c_reset_bss()`. While Exchange is sharing BSS with this app:

- `check_address` and `get_printable_amount` **must not heap-allocate** (no `Vec`, no `String`, no `format!`). Use `arrayvec::ArrayString` and stack arrays.
- `sign_transaction` runs **after** BSS reset, so heap usage is safe there.

The `Bip32Path` wrapper in `src/utils.rs` uses `Vec` and is therefore only legal in normal-mode handlers and in swap's signing phase.

### UI (NBGL)

All supported devices use the New Borrowed Graphics Library:
- Home + settings: `NbglHomeAndSettings` constructed in `src/app_ui/menu.rs`. The same `home` handle is stored on `TxContext` and re-shown via `home.show_and_return()` after each review.
- Address / transaction review: `NbglReview` + `Field` arrays in `src/app_ui/address.rs` and `src/app_ui/sign.rs`.
- Glyphs are loaded with `include_gif!()` gated by `#[cfg(target_os = "...")]`. `build.rs` pre-processes `icons/crab_14x14.gif` + `icons/mask_14x14.gif` into `glyphs/home_nano_nbgl.png` at compile time.

### Settings (NVM)

`src/settings.rs` keeps a 10-byte `AtomicStorage` in the `.nvm_data` link section. Updates go through `AtomicStorage::update()` to survive power loss. Settings are intended to be wired into `NbglHomeAndSettings`' settings switch UI.

## App-specific embedded notes (when editing Rust code)

The generic embedded/Rust rules are imported above; the points below are this app's concrete conventions:

- `#![no_std]` — `extern crate alloc` is declared in `main.rs`; use `alloc::{vec::Vec, string::String, format}`. Never reference `std::`.
- ~24 KB RAM total, default 8 KB heap. Avoid deep recursion, large stack arrays, and gratuitous `clone()`/`collect()`. Prefer iterators and borrowing.
- Handlers return `Result<_, AppSW>` and map SDK errors with `.map_err(|_| AppSW::SomeVariant)`. **Do not** `unwrap()` outside `build.rs`.
- All cryptographic operations go through `ledger_device_sdk` (`Secp256k1::derive_from_path`, `Keccak256`). Never roll your own crypto.
- Treat APDU bytes as untrusted input — validate lengths and structure before parsing.
- Secrets derived from the seed must never be stored, exported, or shown to the user. Sensitive buffers should be zeroed after use.
- Use `ledger_device_sdk::log::debug!` (or `testing::debug_print`) liberally around APDU entry points, crypto calls, and parsing — these are the only way to debug on Speculos.
- Any signing or pubkey-export flow must be gated by an explicit user-validation screen. Do not introduce blind-signing paths.

## Test client layout

- `tests/application_client/` — Python `BoilerplateCommandSender` (APDU construction) and response unpacker, shared by both test suites.
- `tests/standalone/` — pytest suite for normal app launch (dashboard → app).
- `tests/swap/` — pytest suite for the Exchange-driven swap flow; requires Exchange + Ethereum app binaries built into `tests/swap/.test_dependencies/` via `helper_tool_clone_dependencies.py` (host) then `helper_tool_build_dependencies.py` (Docker).
- Tests must construct APDU payloads with named variables and `struct.pack` — no raw hex literals.

## Ledger AI instructions submodule

Generic, cross-repo Ledger embedded-app rules live in the `ledger-app-ai-instructions` submodule
(top-level directory), imported via the `@ledger-app-ai-instructions/CLAUDE.md` line above. It carries
`EMBEDDED.instructions.md`, `RUST.instructions.md`, `C.instructions.md`, `TEST.instructions.md`, and
`REVIEW.instructions.md`. `.github/instructions` is a symlink into `ledger-app-ai-instructions/instructions`
so GitHub Copilot picks up the same files. Run `git submodule update --init` after cloning, or both the
import and the symlink will dangle. Keep this `CLAUDE.md` limited to app-specific guidance; contribute
cross-repo rules to the submodule instead.

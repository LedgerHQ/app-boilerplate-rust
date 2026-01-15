# Ledger Rust App Development Guide

This is a Rust application for Ledger hardware wallets (Nano X, S+, Stax, Flex, Apex P) using the `ledger_device_sdk`.

## Core Development Principles

**Logging**: ALWAYS add `debug_print` logs at key points, especially in complex logic flows:
- Entry/exit of important functions
- Before/after critical operations (crypto, parsing, validation)
- Both success and failure paths
- Key variable values (paths, addresses, amounts) for debugging
- Use `ledger_device_sdk::testing::debug_print()` for all logging

## Architecture

**APDU Command Flow**: The app follows a strict request-response pattern via APDU (Application Protocol Data Unit):
1. `Comm` receives APDU with CLA=0xe0, INS, P1, P2 parameters
2. `Instruction` enum parses APDU header into strongly-typed commands (see `src/main.rs:96-127`)
3. Handler functions in `src/handlers/` process commands and return `Result<(), AppSW>`
4. `AppSW` enum (status words) maps errors to specific hex codes (e.g., `0x6985` = Deny)

**Multi-chunk Transaction Handling**: Large transactions use chunked transmission (see `src/handlers/sign_tx.rs`):
- Chunk 0: BIP32 path only
- Chunks 1-3: Transaction data (max 510 bytes total via `MAX_TRANSACTION_LEN`)
- P2 byte: `0x80` = more chunks, `0x00` = last chunk
- TxContext maintains state between chunks with `raw_tx: Vec<u8>` accumulator

**UI System**: NBGL (New Boilerplate Graphics Library) for all supported devices:
- Home screen via `NbglHomeAndSettings` in `src/app_ui/menu.rs`
- Transaction/address review via `NbglReview` with `Field` arrays
- Device-specific glyphs loaded via `include_gif!()` macro with conditional compilation (`#[cfg(target_os = "stax")]`)

## Build & Test Workflow

**Building** (requires Docker or VS Code extension):
```bash
cargo ledger build nanox -- --features debug -Zunstable-options
# Output: target/{device}/release/app-boilerplate-rust
```

**Testing with Ragger**:
```bash
pip install -r tests/requirements.txt
pytest tests/ --tb=short -v --device {nanosp|nanox|stax|flex}
```
Tests use `BoilerplateCommandSender` client (see `tests/application_client/`) and `scenario_navigator` for UI automation.

**Emulator** (for manual testing):
```bash
speculos --apdu-port 9999 --api-port 5001 --model stax target/stax/release/app-boilerplate-rust
# UI at localhost:5001 (Nano) or via X server (Stax/Flex)
```

## Key Patterns

**Error Handling**: All handlers return `Result<(), AppSW>`. Never use `unwrap()` except in `build.rs`. Map SDK errors to specific `AppSW` variants (e.g., `.map_err(|_| AppSW::KeyDeriveFail)`).

**BIP32 Paths**: Encoded as length byte + 4-byte chunks. `Bip32Path` wrapper in `src/utils.rs` validates format via `TryFrom<&[u8]>`.

**Cryptography**:
- Key derivation: `Secp256k1::derive_from_path()` from `ledger_device_sdk::ecc`
- Hashing: `Keccak256` for Ethereum-style addresses and message signing
- Signature format: DER-encoded + parity byte appended

**Transaction Deserialization**: Uses `serde-json-core` (no_std compatible). The `Tx` struct in `sign_tx.rs` shows memo/to-address pattern with `#[serde(with = "hex::serde")]` for hex-encoded fields.

**Settings Storage**: NVM (non-volatile memory) via `AtomicStorage` in `src/settings.rs`. Linked to `.nvm_data` section. Settings integrate with `NbglHomeAndSettings` switch UI.

**Device-Specific Code**: Pervasive use of `#[cfg(target_os = "...")]` for glyphs, icons, and UI differences between Nano (smaller screens) vs Stax/Flex (touch screens).

## Integration Points

**Python Test Client** (`tests/application_client/`):
- `boilerplate_command_sender.py`: Sends APDUs via Ragger backend
- `boilerplate_response_unpacker.py`: Parses binary responses
- `boilerplate_transaction.py`: Creates JSON transactions matching Rust `Tx` struct

**Build Script** (`build.rs`): Processes GIF icons into PNG glyphs for NBGL at compile time using `image` crate.

**Metadata** (`Cargo.toml`): `[package.metadata.ledger]` section defines app name, icons, derivation paths, and flags per device.

## Critical Constraints

- `#![no_std]` environment: Use `alloc::vec::Vec`, `alloc::format!`, never `std::`
- Stack limits: Avoid deep recursion; prefer iterative patterns
- APDU max size: ~255 bytes per chunk
- Transaction review must call `show_status_and_home_if_needed()` to display NBGL success/failure screens

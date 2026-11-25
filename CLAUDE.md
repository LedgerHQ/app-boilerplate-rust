# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Ledger AI instructions submodule

Generic, cross-repo Ledger embedded-app rules live in the `ledger-app-ai-instructions` submodule
(top-level directory), imported via the `@ledger-app-ai-instructions/CLAUDE.md` line above. It carries
`EMBEDDED.instructions.md`, `RUST.instructions.md`, `C.instructions.md`, `TEST.instructions.md`, and
`REVIEW.instructions.md`. `.github/instructions` is a symlink into `ledger-app-ai-instructions/instructions`
so GitHub Copilot picks up the same files. Run `git submodule update --init` after cloning, or both the
import and the symlink will dangle. Keep this `CLAUDE.md` limited to app-specific guidance; contribute
cross-repo rules to the submodule instead.

@ledger-app-ai-instructions/CLAUDE.md

## What this repo is

Boilerplate Ledger hardware-wallet application written in Rust, targeting Nano X, Nano S+, Stax, Flex, and Apex P. It is a `#![no_std]` / `#![no_main]` embedded binary built against `ledger_device_sdk`, and is intended to be forked as the starting point for new coin apps. Nano S is **not** supported.

## Architecture

### Execution modes

`sample_main` (`src/main.rs`) dispatches based on whether the app was launched from the dashboard or via `os_lib_call` from the Exchange app:

- **Normal mode** (`arg0 == 0`): `normal_main(None)` — shows the home screen, runs the APDU loop, drives the review UI for signing.
- **Swap mode** (`arg0 != 0`): `swap::swap_main(arg0)` dispatches to `check_address` / `get_printable_amount` / `sign_transaction`. Signing reuses `normal_main(Some(&params))` with the UI **bypassed** — Exchange has already validated everything.

### APDU dispatch

The wire protocol (commands, data formats, status words) is specified in `doc/APDU.md`; keep it in sync with `src/main.rs` and the handlers.

1. `Comm` receives an APDU with `CLA=0xe0`; wrong-CLA APDUs are rejected automatically by the SDK (`comm.set_expected_cla(0xe0)`).
2. `Instruction::try_from(ApduHeader)` in `src/main.rs` parses `(INS, P1, P2)` into a strongly-typed enum, INS codes being the `INS_*` constants. Invalid P1/P2 → `AppSW::WrongP1P2`; unknown INS → `AppSW::InsNotSupported`. **All APDU parameter validation lives here, not in the handlers.**
3. `handle_apdu` routes to a handler in `src/handlers/`. Handlers return `Result<CommandResponse, AppSW>`, except `handler_sign_tx`, which returns `AppError` (`src/main.rs`): its `Swap` variant carries a swap rejection, which `normal_main` sends as the 2-byte error code and the message, with `0xC000`.
4. `AppSW` (status word enum in `src/main.rs`) is the single source of truth for error codes returned over the wire (e.g. `0x6985` = Deny, `0x9000` = Ok, app-specific codes are `0xB001..0xB00C`, and swap failures are `0xC000`).
5. After each command, `show_status_and_home_if_needed` decides whether to display an NBGL success/failure screen and return to home — this **must** be called for `GetPubkey { display: true }` and completed `SignTx` flows in normal mode, and **must not** display anything in swap mode.

### Multi-chunk transactions

SIGN_TX (`0x06`) and SIGN_TOKEN_TX (`0x07`) both decode to `Instruction::SignTx`, told apart by its `kind` (`TxKind::Native` / `TxKind::Token`). They share the chunked transmission tracked in `TxContext` (`src/handlers/sign_tx.rs`):

- `P1=0, P2=0x80` — chunk 0 carries the BIP32 path only and starts a new flow, discarding any previous one. Its INS sets the flow kind (`flow_kind`).
- `P1=1..=3, P2=0x80` — intermediate chunks appended to `raw_tx`.
- `P1=1..=3, P2=0x00` — final chunk; triggers parse (`serde_json_core::from_slice` into `NativeTx` or `TokenTx`, per the flow kind), asset resolution (`Tx::into_transfer`), UI review, and signing.
- Chunks must be strictly consecutive after chunk 0 and sent with the INS of chunk 0; anything else returns `AppSW::BadState` (`0xB007`). The flow state (path, `raw_tx`, flow kind) is cleared after every final chunk, every PROVIDE_TOKEN_INFO, and every error, including errors of other commands and of APDU header decoding (`normal_main` clears it for any rejected command).
- `MAX_TRANSACTION_LEN = 510` is enforced on the accumulator; exceeding it returns `AppSW::TxWrongLength`.
- A native transaction's `coin` must be the native ticker (`NATIVE_TICKER` in `src/utils.rs`) in every mode. A token transaction names its token by a 32-byte address only; `token::lookup` (`src/token/`) gives its ticker and decimals from the descriptor slot first, then from the hardcoded list (`src/token/db.rs`, checked at compile time). Nothing in the transaction provides them.
- An unsupported asset (other `coin`, unknown token) returns `AppSW::TxParsingFail` in normal mode and a `SwapError` (`0xC000`) in swap mode (`swap::unsupported_asset_error`).

Hash = Keccak256 of `raw_tx`; signature = secp256k1 deterministic, returned as `[siglen][DER signature][parity]`.

### Signed token descriptors

PROVIDE_TOKEN_INFO (`0x22`, P1 = P2 = 0) → `Instruction::ProvideTokenInfo` → `handlers/provide_token_info.rs`:

- `token::parse_descriptor` (`src/token/dynamic.rs`) calls the SDK's `parse_dynamic_token_tlv` (TLV format + PKI signature with a COIN_META certificate, loaded by the host through CLA `0xB0` and handled by the SDK), then checks version 1, the coin type (`SLIP44_COIN_TYPE`, hardened or not), `DESCRIPTOR_APP_NAME` (per variant, `src/utils.rs`), the ticker and decimals (`TokenInfo::new`), the TUID (a `tlv_generic` table with only tag `0x10`, 32 bytes, required), and that the ticker names no other asset (`ticker_names_other_asset`: not `CRAB`, not a hardcoded ticker at another address, ignoring case). The SDK does not expose the structure type, so it is not checked.
- The handler clears any pending flow and the slot (`TxContext::dynamic_token`) before parsing, so a rejected descriptor also empties the slot. Any failure returns `AppSW::InvalidDynamicToken` (`0xB00C`).
- The slot holds one `DynamicToken` (fixed size, no heap). `clear_flow` does not touch it: it lasts until the app exits. Each swap session (`normal_main`) starts with it empty.
- Parsing allocates (the SDK output holds `Vec`/`String`), so it only runs from the APDU loop: normal mode or swap's signing phase, never from `check_address` / `get_printable_amount`.

### Token swap

`swap::check_swap_asset` compares the resolved asset with the Exchange coin configuration (`parse_coin_config`):

- Only an **empty** configuration is the native coin (`is_native_coin_config`); any other one is a token, even `CRAB`/9. A native swap needs `Asset::Native`, a token swap needs `Asset::Token` with exactly the configuration's ticker and decimals (else `ErrorWrongAmount` / `SwapAppErrorCode::WrongTokenInfo`). A kind mismatch is `ErrorWrongMethod`.
- The configuration carries no token address: matching the ticker is enough only because at most one address resolves to a ticker (unique hardcoded tickers, one descriptor slot, `ticker_names_other_asset`). Keep that invariant when changing the token list or the descriptor checks.
- A descriptor for a swapped token must be sent during the swap's signing phase, before the SIGN_TOKEN_TX: the slot starts empty in each swap session.
- `get_printable_amount` always formats fees (`params.is_fee`) in `CRAB` with 9 decimals, even for a token swap.

### Swap memory constraint (critical)

`src/swap.rs` runs **before** the SDK calls `c_reset_bss()`. While Exchange is sharing BSS with this app:

- `check_address` and `get_printable_amount` **must not heap-allocate** (no `Vec`, no `String`, no `format!`). Use `arrayvec::ArrayString` and stack arrays. `token::lookup`, `TokenInfo` and `DynamicToken` do not allocate; `token::parse_descriptor` does.
- `sign_transaction` runs **after** BSS reset, so heap usage is safe there.

The `Bip32Path` wrapper in `src/utils.rs` only accepts paths under `m/44'/1'` with at most 10 components (`is_allowed_path`, also applied by swap's `check_address`), returning `AppSW::InvalidPath` otherwise. It uses `Vec` and is therefore only legal in normal-mode handlers and in swap's signing phase.

### UI (NBGL)

All supported devices use the New Bolos Graphics Library:
- Home + settings: `NbglHomeAndSettings` constructed in `src/app_ui/menu.rs`. The same `home` handle is stored on `TxContext` and re-shown via `home.show_and_return()` after each review.
- Address / transaction review: `NbglReview` + `Field` arrays in `src/app_ui/address.rs` and `src/app_ui/sign.rs`.
- Glyphs are loaded with `include_gif!()` gated by `#[cfg(target_os = "...")]`. `build.rs` pre-processes `icons/crab_14x14.gif` + `icons/mask_14x14.gif` into `glyphs/home_nano_nbgl.png` at compile time.

### Settings (NVM)

`src/settings.rs` keeps a 10-byte `AtomicStorage` in the `.nvm_data` link section. Updates go through `AtomicStorage::update()` to survive power loss. Settings are wired into `NbglHomeAndSettings`' settings switch UI (`src/app_ui/menu.rs`). The only switch, `SETTING_SHOW_PATH`, adds the derivation path to the transaction review.

## App-specific embedded notes (when editing Rust code)

The generic embedded/Rust rules are imported above; the points below are this app's concrete conventions:

- `#![no_std]` — `extern crate alloc` is declared in `main.rs`; use `alloc::{vec::Vec, string::String, format}`. Never reference `std::`.
- Treat APDU bytes as untrusted input — validate lengths and structure before parsing.
- Secrets derived from the seed must never be stored, exported, or shown to the user. Sensitive buffers should be zeroed after use.
- Use `ledger_device_sdk::log::debug!` liberally around APDU entry points, crypto calls, and parsing — these are the only way to debug on Speculos.
- Any signing or pubkey-export flow must be gated by an explicit user-validation screen. Do not introduce blind-signing paths.

## Test client layout

- `tests/application_client/` — Python `BoilerplateCommandSender` (APDU construction) and response unpacker, shared by both test suites.
- `tests/standalone/` — pytest suite for normal app launch (dashboard → app).
- `tests/swap/` — pytest suite for the Exchange-driven swap flow; requires Exchange + Ethereum app binaries built into `tests/swap/.test_dependencies/` via `helper_tool_clone_dependencies.py` (host) then `helper_tool_build_dependencies.py` (Docker).
- Tests must construct APDU payloads with named variables and `struct.pack` — no raw hex literals.

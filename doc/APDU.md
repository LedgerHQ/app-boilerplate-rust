# APDU protocol

The application exchanges ISO 7816-4 style APDUs with the host. Every command uses class `CLA = 0xE0`;
any other class is rejected with `0x6E00`.

| INS    | Command                            | P1                          | P2                        |
|--------|------------------------------------|-----------------------------|---------------------------|
| `0x03` | [GET_VERSION](#get_version)        | `0x00`                      | `0x00`                    |
| `0x04` | [GET_APP_NAME](#get_app_name)      | `0x00`                      | `0x00`                    |
| `0x05` | [GET_PUBLIC_KEY](#get_public_key)  | `0x00` silent, `0x01` display | `0x00`                  |
| `0x06` | [SIGN_TX](#sign_tx)                | chunk index `0x00`..`0x03`  | `0x80` more, `0x00` last  |
| `0x07` | [SIGN_TOKEN_TX](#sign_token_tx)    | chunk index `0x00`..`0x03`  | `0x80` more, `0x00` last  |
| `0x22` | [PROVIDE_TOKEN_INFO](#provide_token_info) | `0x00`               | `0x00`                    |

Any other P1/P2 combination returns `0x6A86`, any other INS returns `0x6D00`.

## Derivation paths

Commands taking a BIP32 path encode it as one byte holding the number of components `n`,
followed by `n` big-endian `u32` components, hardened ones having their most significant bit set.

| Field      | Size      | Description                    |
|------------|-----------|--------------------------------|
| `n`        | 1         | Number of path components      |
| components | `4 * n`   | Big-endian path components     |

A length that does not match `n` returns `0x6E03`. Only paths starting with `m/44'/1'` and having
at most 10 components are accepted, others return `0xB00B`.

## GET_VERSION

Returns the application version.

| Response field | Size |
|----------------|------|
| Major          | 1    |
| Minor          | 1    |
| Patch          | 1    |

## GET_APP_NAME

Returns the application name as ASCII bytes, without length prefix or terminator.

## GET_PUBLIC_KEY

Data: a [derivation path](#derivation-paths).

With `P1 = 0x01`, the address derived from the key is displayed and must be approved by the user;
a refusal returns `0x6985`.

| Response field        | Size |
|-----------------------|------|
| Public key length     | 1    |
| Uncompressed public key (`0x04 ‖ X ‖ Y`) | 65 |
| Chain code length     | 1    |
| Chain code            | 32   |

The address is the last 20 bytes of the Keccak-256 hash of the public key without its `0x04` prefix.

## SIGN_TX

The transaction is sent in several chunks, P1 being the chunk index:

1. Chunk `0x00`, with `P2 = 0x80`: a [derivation path](#derivation-paths). It starts a new flow and
   discards any previous one.
2. Chunks `0x01` to `0x03`: consecutive parts of the serialized transaction, `P2 = 0x80` for all
   but the last one, sent with `P2 = 0x00`.

Chunks must be strictly consecutive after chunk `0x00` and sent with the INS of chunk `0x00`; any
other sequence returns `0xB007`. The flow is cleared after the last chunk, after any command answered
with an error, whatever its INS, and after any [PROVIDE_TOKEN_INFO](#provide_token_info), so a new
flow must start with chunk `0x00`. APDUs with a wrong class or a malformed header are
rejected before reaching the application and leave the flow untouched. The serialized transaction
is at most 510 bytes, beyond which `0xB004` is returned.

### Transaction format

The transaction is a JSON object with exactly these fields, any other field being rejected:

| Field   | Type                  | Description                                          |
|---------|-----------------------|------------------------------------------------------|
| `nonce` | unsigned 64-bit       | Transaction nonce                                    |
| `coin`  | string                | Asset ticker, always `CRAB`                          |
| `value` | unsigned 64-bit       | Amount in base units, displayed with 9 decimals      |
| `to`    | string, 40 hex digits | 20-byte destination address                          |
| `memo`  | string                | Free text                                            |

`coin` and `memo` must be printable ASCII without backslash, and `coin` cannot be empty. A
transaction that cannot be parsed or validated returns `0xB005`. Tokens are transferred with
[SIGN_TOKEN_TX](#sign_token_tx), in swaps too: any other `coin` than `CRAB` returns `0xB005`, or
`0xC000` in [swap mode](#swap-mode).

### Review and signature

The device displays the amount, the destination and the memo, plus the derivation path when the
"Derivation path" setting is enabled. A refusal returns `0x6985`.

On approval, the Keccak-256 hash of the serialized transaction, exactly as received, is signed
with deterministic ECDSA over secp256k1:

| Response field      | Size        |
|---------------------|-------------|
| Signature length    | 1           |
| DER signature       | variable    |
| Parity of `R.y`     | 1           |

### Swap mode

When the Exchange application starts the app to finalize a swap, the review is skipped and the
transaction must match the swap approved in Exchange: asset, amount, destination, and memo equal to
the destination extra ID; swaps with fees are rejected. A mismatch, an unsupported asset, or a
transaction that cannot be parsed returns `0xC000`, followed by a two-byte error code and a message
in the response data, and ends the swap.

The asset comes from the coin configuration that Exchange gets from the Ledger CAL:

- An empty configuration designates the native coin: the transaction is a SIGN_TX of `CRAB`.
- Any other configuration designates a token, even with the ticker `CRAB`: the transaction is a
  [SIGN_TOKEN_TX](#sign_token_tx). The app finds the ticker and the decimals of its token as in
  normal mode, from the [PROVIDE_TOKEN_INFO](#provide_token_info) descriptor sent during the swap,
  then from the hardcoded list. They must be equal to those of the configuration.

A SIGN_TX for a token swap, or a SIGN_TOKEN_TX for a native coin swap, is rejected. So is an
unknown token, and a SIGN_TX with a `coin` other than `CRAB`.

## SIGN_TOKEN_TX

Signs a token transfer. The chunks, the flow rules, the review and the signature are those of
[SIGN_TX](#sign_tx); only the transaction format differs. A flow started with SIGN_TOKEN_TX must be
continued with SIGN_TOKEN_TX, and a flow started with SIGN_TX with SIGN_TX, or `0xB007` is returned.

### Token transaction format

The transaction is a JSON object with exactly these fields, any other field (`coin` included) being
rejected:

| Field   | Type                  | Description                                          |
|---------|-----------------------|------------------------------------------------------|
| `nonce` | unsigned 64-bit       | Transaction nonce                                    |
| `token` | string, 64 hex digits | 32-byte address of the token                         |
| `value` | unsigned 64-bit       | Amount in base units of the token                    |
| `to`    | string, 40 hex digits | 20-byte destination address                          |
| `memo`  | string                | Free text                                            |

`memo` must be printable ASCII without backslash. A transaction that cannot be parsed returns
`0xB005`.

### Token lookup

The transaction carries no ticker and no decimals: the app looks the token address up, in this
order (see [token support](TOKENS.md)):

1. The token of the last descriptor accepted by [PROVIDE_TOKEN_INFO](#provide_token_info).
2. The hardcoded token list.

An address found in neither returns `0xB005`, before any review, or `0xC000` in
[swap mode](#swap-mode).

### Token review

The device displays the token ticker, the amount with the ticker and the decimals of the token,
the destination and the memo, plus the derivation path when the "Derivation path" setting is
enabled. The review titles name the token ticker. A refusal returns `0x6985`.

In [swap mode](#swap-mode) the review is skipped and the transaction is checked against the swap.

## PROVIDE_TOKEN_INFO

Gives the ticker and decimals of a token, in a descriptor signed by the Ledger CAL (Crypto Assets
List). Data: the descriptor, in a single APDU. The response has no data.

### Certificate

The app verifies the descriptor signature with the PKI certificate that the host loads beforehand
with the BOLOS command `CLA = 0xB0`, `INS = 0x06`, `P1 = 0x08` (key usage COIN_META), which the OS
handles. Without such a certificate, every descriptor is rejected.

### Descriptor format

The descriptor is a list of TLV fields: a tag, a length and a value, the tag and the length being
DER encoded. Each tag must be present exactly once, and any other tag is rejected.

| Tag    | Field            | Value                                                               |
|--------|------------------|---------------------------------------------------------------------|
| `0x01` | Structure type   | `0x90`, dynamic token                                               |
| `0x02` | Version          | `1`                                                                 |
| `0x03` | Coin type        | 1 to 4 big-endian bytes: `1` or `0x80000001`, SLIP-0044 coin type of the app |
| `0x04` | App name         | `Rust Boilerplate` (`Rust Testnet BP` and `Rust Betanet BP` for the variants) |
| `0x05` | Ticker           | 1 to 12 printable ASCII characters without spaces. Not `CRAB`, nor the ticker of a token of the hardcoded list at another address, ignoring case |
| `0x06` | Magnitude        | Decimals of the token, 30 at most                                   |
| `0x07` | TUID             | Token identifier, see below                                         |
| `0x08` | Signature        | DER ECDSA signature over secp256k1 of the SHA-256 hash of all the other fields, as sent |

The TUID (Token Unique IDentifier) is itself a TLV list with a single field: tag `0x10`, holding the
32-byte token address. Any other tag is rejected.

The structure type is required, but its value is not checked by the app (see
[token support](TOKENS.md#2-signed-token-descriptors)).

### Descriptor slot

The app keeps the token of one descriptor. Each PROVIDE_TOKEN_INFO whose P1 and P2 are accepted
first empties that slot, then fills it if the descriptor is valid: a rejected descriptor also
discards the previous one, and returns `0xB00C`. A command rejected for its P1 or P2 (`0x6A86`)
leaves the slot as it is.

PROVIDE_TOKEN_INFO also ends any pending [SIGN_TX](#sign_tx) or [SIGN_TOKEN_TX](#sign_token_tx)
flow, so the descriptor of a transaction is sent before its chunk `0x00`. The slot is kept across
signing flows, until another PROVIDE_TOKEN_INFO or until the app exits. In swap mode the app starts
with an empty slot: the host sends the certificate and the descriptor of a swapped token after
Exchange starts the app, before the SIGN_TOKEN_TX.

## Status words

| Status word | Name                 | Meaning                                                   |
|-------------|----------------------|-----------------------------------------------------------|
| `0x9000`    | Ok                   | Success                                                   |
| `0x6985`    | Deny                 | Rejected by the user                                      |
| `0x6A86`    | WrongP1P2            | Invalid P1 or P2                                          |
| `0x6D00`    | InsNotSupported      | Unknown instruction                                       |
| `0x6E00`    | ClaNotSupported      | Unknown class                                             |
| `0x6E03`    | WrongApduLength      | Data length inconsistent with the command                 |
| `0x6F00`    | CommError            | Response could not be built                               |
| `0xB001`    | TxDisplayFail        | Transaction could not be displayed                        |
| `0xB002`    | AddrDisplayFail      | Address could not be displayed                            |
| `0xB004`    | TxWrongLength        | Transaction longer than 510 bytes                         |
| `0xB005`    | TxParsingFail        | Transaction malformed or not supported                    |
| `0xB006`    | TxHashFail           | Transaction hashing failed                                |
| `0xB007`    | BadState             | SIGN_TX or SIGN_TOKEN_TX chunk out of sequence            |
| `0xB008`    | TxSignFail           | Signature failed                                          |
| `0xB009`    | KeyDeriveFail        | Key or address derivation failed                          |
| `0xB00A`    | VersionParsingFail   | Application version could not be encoded                  |
| `0xB00B`    | InvalidPath          | Derivation path outside the scope of the application      |
| `0xB00C`    | InvalidDynamicToken  | Token descriptor rejected                                 |
| `0xC000`    | SwapFail             | Transaction does not match the swap                       |

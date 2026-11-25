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
other sequence returns `0xB007`. The flow is cleared after the last chunk and after any command answered with an error, whatever its
INS, so a new flow must start with chunk `0x00`. APDUs with a wrong class or a malformed header are
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
[SIGN_TOKEN_TX](#sign_token_tx): any other `coin` than `CRAB` returns `0xB005`, or `0xC000` in
[swap mode](#swap-mode).

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
the destination extra ID; swaps with fees are rejected. Only the native coin can be swapped: a
SIGN_TOKEN_TX transaction is rejected. A mismatch or an unsupported asset returns `0xC000`, followed
by a two-byte error code and a message in the response data.

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

The transaction carries no ticker and no decimals: the app looks the token address up in its
hardcoded token list (see [token support](TOKENS.md)). An address that is not in the list returns
`0xB005`, before any review.

### Token review

The device displays the token ticker, the amount with the ticker and the decimals of the token,
the destination and the memo, plus the derivation path when the "Derivation path" setting is
enabled. The review titles name the token ticker. A refusal returns `0x6985`.

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
| `0xC000`    | SwapFail             | Transaction does not match the swap                       |

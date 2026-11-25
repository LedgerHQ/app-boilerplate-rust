# Token support

Besides its native coin `CRAB`, the application signs transfers of tokens with the
[SIGN_TOKEN_TX](APDU.md#sign_token_tx) command.

## Token identity

A token transaction names its token by a 32-byte address only. The ticker and the decimals that the
device displays come from what the application knows of that address, never from the transaction:
a host cannot make a token look like another one, or change how its amounts read.

A token the application does not know cannot be reviewed, so its transfer is rejected.

## Levels of token support

A Ledger application can support tokens at three levels. Each level needs more coordination with
Ledger than the one before it.

### 1. Hardcoded token list

The application holds a list of known tokens, in [`src/token/db.rs`](../src/token/db.rs). This is
the level that this application implements.

- Use it when the chain has a small set of tokens that changes rarely.
- No coordination with Ledger is needed: add the tokens to the list and release the application.
- Adding a token requires a new release of the application.

Each entry maps an address to a ticker and decimals:

```rust
TokenEntry {
    address: [0x01, 0x23, /* ... 32 bytes in total */],
    ticker: "USDC",
    decimals: 12,
},
```

The build checks the list:

- Each ticker has 1 to 12 printable ASCII characters without spaces.
- The decimals are 30 at most.
- No two entries have the same address, or the same ticker ignoring case.
- No ticker is `CRAB`, ignoring case.

The review identifies a token by its ticker only, so two tokens with the same ticker would look the
same to the user. The entries of this repository are examples, as the chain of the application does
not exist.

### 2. Signed token descriptors

The host sends a token descriptor, signed by Ledger, before the transaction. The application checks
the signature against a certificate of the Ledger PKI and uses the token for the next transactions.

- Use it when the chain has many tokens, or when tokens are added often.
- It requires coordination with Ledger, which signs the descriptors.
- New tokens need no release of the application.

This application does not implement this level.

### 3. Token swap

The Exchange application can swap a token when the token is in the Ledger CAL (Crypto Assets List),
which gives Exchange its ticker and decimals. Adding a token to the CAL requires coordination with
Ledger.

This application only swaps its native coin: it rejects token transfers in swap mode.

The CAL entry of the native coin must have an empty sub coin configuration. The application treats
any other configuration as a token, even one with the ticker `CRAB` and 9 decimals: the CAL can
hold a token with the same ticker as the native coin.

## Token lookup order

1. Hardcoded token list.
2. Unknown token: the transaction is rejected with `0xB005`.

In swap mode, every token transfer is rejected with `0xC000`, known or not.

## Testing

Send token transactions for the tokens of the hardcoded list, see
[`tests/standalone/test_sign_token_cmd.py`](../tests/standalone/test_sign_token_cmd.py). The test
token addresses are in
[`tests/application_client/boilerplate_tokens.py`](../tests/application_client/boilerplate_tokens.py).

# Token support

Besides its native coin `CRAB`, the application signs transfers of tokens with the
[SIGN_TOKEN_TX](APDU.md#sign_token_tx) command.

## Token identity

A token transaction names its token by a 32-byte address only. The ticker and the decimals that the
device displays come from what the application knows of that address, never from the transaction:
the hardcoded token list, or a token descriptor signed by Ledger. A host cannot make up the ticker
or the decimals of a token. It can only choose which signed descriptor to send, and a descriptor
cannot use the ticker of the native coin or of another token of the list.

A token the application does not know cannot be reviewed, so its transfer is rejected.

## Levels of token support

A Ledger application can support tokens at three levels. Each level needs more coordination with
Ledger than the one before it.

### 1. Hardcoded token list

The application holds a list of known tokens, in [`src/token/db.rs`](../src/token/db.rs).

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

The review identifies a token by its ticker only, so two tokens of the list with the same ticker
would look the same to the user. The entries of this repository are examples, as the chain of the
application does not exist.

### 2. Signed token descriptors

The host sends a token descriptor, signed by Ledger, with the
[PROVIDE_TOKEN_INFO](APDU.md#provide_token_info) command before the transaction. The application
checks the signature with a certificate of the Ledger PKI and uses the token for the next
transactions.

- Use it when the chain has many tokens, or when tokens are added often.
- It requires coordination with Ledger: the descriptors are signed by the Ledger CAL (Crypto Assets
  List), so the tokens of the chain must be in the CAL, and Ledger Wallet must send the descriptors.
- New tokens need no release of the application.

The descriptor gives the ticker and the decimals of one token address. The SDK function
`ledger_device_sdk::tlv::parse_dynamic_token_tlv` parses it and verifies its signature. The
application, in [`src/token/dynamic.rs`](../src/token/dynamic.rs), then checks that:

- The version is 1.
- The coin type is the SLIP-0044 coin type of the application, hardened or not.
- The application name is the one of the application (`DESCRIPTOR_APP_NAME` in
  [`src/utils.rs`](../src/utils.rs), to update when a fork renames the application).
- The ticker has 1 to 12 printable ASCII characters without spaces, and the decimals are 30 at most.
- The ticker is not `CRAB`, nor the ticker of a token of the hardcoded list at another address,
  ignoring case. The review identifies a token by its ticker only, so the descriptor token would
  look like that other asset.
- The TUID (Token Unique IDentifier) holds the 32-byte token address, and nothing else.

The TUID format is specific to each application. Here it only holds the token address, with tag
`0x10`. An application whose tokens are identified by more than an address adds a tag for each
field.

The SDK does not give the value of the structure type field, so the application cannot check it. The
signature still binds the descriptor to the CAL, as only a key with a COIN_META certificate can make
it.

The application keeps one descriptor, in RAM, until it exits or receives another descriptor. Each
PROVIDE_TOKEN_INFO command first discards the previous descriptor, and ends any pending signing
flow.

Ledger signs the descriptors with a CAL key, which a certificate of the Ledger PKI authorizes. The
tests use a test key whose certificate only Speculos accepts, see
[`tests/application_client/signing_partners.py`](../tests/application_client/signing_partners.py).
A real device does not accept the test certificate, so it rejects the descriptors of the tests.

### 3. Token swap

The Exchange application can swap a token when the token is in the Ledger CAL (Crypto Assets List),
which gives Exchange its ticker and decimals. Adding a token to the CAL requires coordination with
Ledger.

- Use it when users must be able to swap the tokens of the chain from Ledger Wallet.
- It requires coordination with Ledger: the token must be in the CAL, with the name of the
  application.
- It works with both levels above: the token can come from the hardcoded list or from a descriptor.

The CAL entry of the native coin must have an empty sub coin configuration. The application treats
any other configuration as a token, even one with the ticker `CRAB` and 9 decimals: the CAL can
hold a token with the same ticker as the native coin.

Exchange gives the application the coin configuration of the swap: the ticker and the decimals of
the token, but not its address. To finalize the swap, the host sends a
[SIGN_TOKEN_TX](APDU.md#sign_token_tx) transaction, which names the token by its address. The
application finds the token as in normal mode (see the [lookup order](#token-lookup-order)), then
checks that its ticker and decimals are those of the configuration. The ticker is enough to identify
the token, as only one address at a time can have a given ticker: the tickers of the hardcoded list
are unique, the application keeps one descriptor, and a descriptor cannot use the ticker of a token
of the list at another address.

A swapped token known from a descriptor needs its descriptor during the swap. Exchange starts the
application with no descriptor, so the host sends the certificate and the descriptor before the
transaction, as in normal mode.

The application rejects with `0xC000`:

- a native coin transfer (SIGN_TX) for a token swap, and a token transfer for a native coin swap;
- a token transfer of an unknown token, or of a token with another ticker or other decimals than the
  configuration.

The fees of a swap are always shown in `CRAB`, the coin that pays them, even for a token swap.

## Token lookup order

1. Token of the last accepted descriptor. It comes first, so a descriptor can replace the ticker and
   decimals of a token of the hardcoded list, as Ledger signed them.
2. Hardcoded token list.
3. Unknown token: the transaction is rejected with `0xB005`, or with `0xC000` in swap mode.

## Testing

Send token transactions for the tokens of the hardcoded list, see
[`tests/standalone/test_sign_token_cmd.py`](../tests/standalone/test_sign_token_cmd.py). The test
token addresses are in
[`tests/application_client/boilerplate_tokens.py`](../tests/application_client/boilerplate_tokens.py).

To test descriptors on Speculos, load the test certificate, then send descriptors signed with the
test key, see
[`tests/standalone/test_provide_token_info_cmd.py`](../tests/standalone/test_provide_token_info_cmd.py)
and
[`tests/application_client/boilerplate_token_descriptor.py`](../tests/application_client/boilerplate_token_descriptor.py).

The token swap tests, in [`tests/swap/test_boilerplate.py`](../tests/swap/test_boilerplate.py), swap
a token of the hardcoded list (`BoilerplateTokenTests`) and a token known from a descriptor
(`BoilerplateDynamicTokenTests`). Their mock CAL configurations are in
[`tests/swap/cal_helper.py`](../tests/swap/cal_helper.py), and must carry the name of the
application.

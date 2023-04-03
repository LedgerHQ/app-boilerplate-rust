# Rust Nano S Application 

A simple application that receives a message, displays it, and requests user approval to sign. Can also display an example menu.

## Building

### Prerequisites

This project will try to build [nanos-secure-sdk](https://github.com/LedgerHQ/nanos-secure-sdk) as a dependency. You should follow toolchain preparation [instructions](https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/README.md#usage) to get everything working.

In order to build easily, you should use [cargo-ledger](https://github.com/LedgerHQ/cargo-ledger.git).

To run your app in an emulator, use [Speculos](https://github.com/LedgerHQ/speculos).

You can build for `nanosplus` on either Windows or Linux with a simple `cargo build` or `cargo build --release`.

## Loading

You can use [cargo-ledger](https://github.com/LedgerHQ/cargo-ledger.git) which builds, outputs a `hex` file and a manifest file for `ledgerctl`, and loads it on a device in a single `cargo ledger build <target> --load` command in your app directory.

Some options of the manifest file can be configured directly in `Cargo.toml` under a custom section:

```yaml
[package.metadata.nanos]
curve =  ["secp256k1"]
flags = "0x40"
icon = "btc.gif"
```

## Testing

One can for example use [speculos](https://github.com/LedgerHQ/speculos)

`cargo run --release` defaults to running speculos on the generated binary with the appropriate flags, if `speculos` is in your `PATH`.

There is a small test script that sends some of the available commands in `test/test_cmds.py`, or raw APDUs that can be used with `ledgerctl`.

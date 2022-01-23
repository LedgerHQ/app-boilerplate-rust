# Rust Nano S Application 

A simple application that receives a message, displays it, and requests user approval to sign. Can also display an example menu.

## Building

### Prerequisites

This project will try to build [nanos-secure-sdk](https://github.com/LedgerHQ/nanos-secure-sdk), so you will need:

#### Linux

1. A standard ARM gcc (`sudo apt-get install gcc-arm-none-eabi binutils-arm-none-eabi`)
2. Cross compilation headers (`sudo apt-get install gcc-multilib`)
2. Python3 (`sudo apt-get install python3`)
3. Pip3 (`sudo apt-get install python3-pip`)

#### Windows

1. install [Clang](http://releases.llvm.org/download.html)
2. install an [ARM GCC toolchain](https://developer.arm.com/tools-and-software/open-source-software/developer-tools/gnu-toolchain/gnu-rm/downloads)
3. [Python](https://www.python.org/)

#### macOS

1. Install `Apple Clang 12` from [Command Line Tools for Xcode 12.5.1](https://download.developer.apple.com/Developer_Tools/Command_Line_Tools_for_Xcode_12.5.1/Command_Line_Tools_for_Xcode_12.5.1.dmg) - `Apple Clang 13`, `Homebrew Clang 12` etc. won't work
2. Install [xpm](https://xpack.github.io/install/#install-xpm) & the [xPack GNU Arm Embedded GCC toolchain](https://xpack.github.io/arm-none-eabi-gcc/install/#macos_1) with `x86_64` and `arm64` support
3. Further instructions: [issues/14#issuecomment-1019378659](https://github.com/LedgerHQ/rust-app/issues/14#issuecomment-1019378659)

Other things you will need:
- [Cargo-ledger](https://github.com/LedgerHQ/cargo-ledger.git)
- [Speculos](https://github.com/LedgerHQ/speculos) (make sure you add speculos.py to your PATH by running `export PATH=/path/to/speculos:$PATH`)
- The correct target for rustc: `rustup target add thumbv6m-none-eabi`

You can build on either Windows or Linux with a simple `cargo build` or `cargo build --release`.
It currently builds on stable.

## Loading

You can use [cargo-ledger](https://github.com/LedgerHQ/cargo-ledger.git) which builds, outputs a `hex` file and a manifest file for `ledgerctl`, and loads it on a device in a single `cargo ledger load` command in your app directory.

Some options of the manifest file can be configured directly in `Cargo.toml` under a custom section:

```yaml
[package.metadata.nanos]
curve = "secp256k1"
flags = "0x40"
icon = "btc.gif"
```

## Testing

One can for example use [speculos](https://github.com/LedgerHQ/speculos)

`cargo run --release` defaults to running speculos on the generated binary with the appropriate flags, if `speculos.py` is in your `PATH`.

There is a small test script that sends some of the available commands in `test/test_cmds.py`, or raw APDUs that can be used with `ledgerctl`.

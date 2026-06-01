# Ledger Rust Boilerplate Application

![Rule enforcer](https://github.com/LedgerHQ/app-boilerplate-rust/actions/workflows/guidelines_enforcer.yml/badge.svg) ![Build and tests](https://github.com/LedgerHQ/app-boilerplate-rust/actions/workflows/build_and_functional_tests.yml/badge.svg)

This is a boilerplate application written in Rust which can be forked to start a new project for the Ledger devices.

* Implements standard features (display address, transaction signature...),
* Has functional tests using [Ragger](https://github.com/LedgerHQ/ragger),
* Has CI workflows mandatory for app deployment in the Ledger store.

### Links

* 📚 [Developer's documentation](https://developers.ledger.com/docs/device-app/getting-started)<br/>
* 🗣️ [Ledger's Discord server](https://discord.gg/Ledger)

## Quick start guide

### With VS Code

You can quickly setup a development environment on any platform (macOS, Linux or Windows) to build and test your application with [Ledger's VS Code extension](https://marketplace.visualstudio.com/items?itemName=LedgerHQ.ledger-dev-tools).

By using Ledger's own developer tools [Docker image](https://github.com/LedgerHQ/ledger-app-builder/pkgs/container/ledger-app-builder%2Fledger-app-dev-tools), the extension allows you to **build** your apps with the latest SDK, **test** them on **Speculos** and **load** them on any supported device.

* Install and run [Docker](https://www.docker.com/products/docker-desktop/).
* Make sure you have an X11 server running :
  * On Ubuntu Linux, it should be running by default.
  * On macOS, install and launch [XQuartz](https://www.xquartz.org/) (make sure to go to XQuartz > Preferences > Security and check "Allow client connections").
  * On Windows, install and launch [VcXsrv](https://sourceforge.net/projects/vcxsrv/) (make sure to configure it to disable access control).
* Install [VScode](https://code.visualstudio.com/download) and add [Ledger's extension](https://marketplace.visualstudio.com/items?itemName=LedgerHQ.ledger-dev-tools).
* Open a terminal and clone `app-boilerplate-rust` with `git clone git@github.com:LedgerHQ/app-boilerplate-rust.git`.
* Open the `app-boilerplate-rust` folder with VSCode.
* Use Ledger extension's sidebar menu or open the tasks menu with `ctrl + shift + b` (`command + shift + b` on a Mac) to conveniently execute actions :
  * **Build** the app for the device model of your choice with `Build`.
  * **Test** your binary on the [Speculos emulator](https://github.com/LedgerHQ/speculos) with `Run with emulator`.
  * You can also **run functional tests**, load the app on a physical device, and more.

ℹ️ The terminal tab of VSCode will show you what commands the extension runs behind the scene.

## With a terminal

### Prerequisites

If you do not wish to use the [VS Code extension](#with-vs-code), you can follow the following steps to setup a development environment on Linux, Windows or MacOS.

* The [ledger-app-dev-tools](https://github.com/LedgerHQ/ledger-app-builder/pkgs/container/ledger-app-builder%2Fledger-app-dev-tools) Docker image contains all the required tools and libraries to build, test and load an application on a device. You can download it from the ghcr.io docker repository:
```shell
docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
```
* Make sure you have an X11 server running :
  * On Ubuntu Linux, it should be running by default.
  * On macOS, install and launch [XQuartz](https://www.xquartz.org/) (make sure to go to XQuartz > Preferences > Security and check "Allow client connections").
  * On Windows, install and launch [VcXsrv](https://sourceforge.net/projects/vcxsrv/) (make sure to configure it to disable access control).

There are two ways to use the container, depending on what you are doing.

#### Interactive shell (recommended for development)

Drop into a shell inside the container once, then run as many `cargo ledger build` / `pytest` commands as you like. The bind-mount (`-v .../app`) keeps the build artifacts on your host. The extra `--privileged`, `--publish` and X11 (`DISPLAY`, `/tmp/.X11-unix`) flags are only needed if you intend to run Speculos — they are harmless otherwise.

Run the command matching your OS from the directory of the application (`git` repository):
  * Linux (Ubuntu): 
  ```shell
  sudo docker run --rm -ti --privileged -v "/dev/bus/usb:/dev/bus/usb" -v "$(realpath .):/app" --publish 5001:5001 --publish 9999:9999 -e DISPLAY=$DISPLAY -v '/tmp/.X11-unix:/tmp/.X11-unix' ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
  ```
  * macOS:
  ```shell
  docker run  --rm -ti -v "$(pwd -P):/app" --publish 5001:5001 --publish 9999:9999 -e DISPLAY='host.docker.internal:0' ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
  ```
  * Windows (with PowerShell):
  ```shell
  docker run --rm -ti --privileged -v "$(Get-Location):/app" -e DISPLAY='host.docker.internal:0' --publish 5001:5001 --publish 9999:9999 ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
  ```

The application's code is mounted at `/app` inside the container. Once in the shell, proceed to the [Building](#building) and [Testing](#testing) steps below — run those commands inside the container.

#### One-shot command (handy for CI or a quick build)

Instead of opening a shell, append the command to run to `docker run` and the container exits when it finishes. The repository is still mounted at `/app`, so artifacts land in your host `target/` directory. For example, to build for Flex (macOS / Linux):

```shell
docker run --rm -v "$(pwd -P):/app" ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest cargo ledger build flex
```

Any of the commands from [Building](#building) and [Testing](#testing) can be run this way. Add the `--privileged`, `--publish` and X11 flags shown above if the command needs Speculos (e.g. functional tests).

### Building

You can build the boilerplate with the following command executed in the root directory of the app.
```bash
cargo ledger build nanox
```
This command will build the app for the Nano X, but you can use any supported device (`nanox`, `nanosplus`, `stax`, `flex`, `apex_p`).

> ℹ️ `.cargo/config.toml` sets `apex_p` as the default cargo target, so a bare `cargo ledger build` builds for Apex P. Always pass the device explicitly to be sure.

### Testing
#### Ragger functional tests
This boilerplate app comes with functional tests implemented with Ledger's [Ragger](https://github.com/LedgerHQ/ragger) test framework. There are two suites:
* `tests/standalone/` — normal app launch (dashboard → app).
* `tests/swap/` — the Exchange-driven swap flow (requires Exchange + Ethereum app binaries, see [tests/swap/README.md](tests/swap/README.md)).

* Install the tests requirements
```bash
pip install -r tests/standalone/requirements.txt
```
* Run the standalone functional tests :

```shell
pytest tests/standalone/ --tb=short -v --device {nanosp | nanox | stax | flex | apex_p}
```

> ℹ️ Speculos uses `nanosp` for Nano S+ (whereas `ledger_app.toml` lists it as `nanos+`).
#### Emulator
You can also run the app directly on the [Speculos emulator](https://github.com/LedgerHQ/speculos) from the Docker container
#### Nano S+ or X
```bash
speculos --apdu-port 9999 --api-port 5001 --display headless target/nanosplus/release/app-boilerplate-rust
```
:warning: UI is displayed on `localhost:5001`
#### Stax, Flex or Apex P
```bash
speculos --apdu-port 9999 --api-port 5001 target/stax/release/app-boilerplate-rust
```

:warning: UI is displayed by your X server

You can then send APDU using `ledgercomm` (`pip install ledgercomm`):
```
ledgercomm-send file test.apdu
```
### Loading on device
Recent versions of [cargo-ledger](https://github.com/LedgerHQ/cargo-ledger) no longer emit a `ledgerctl` JSON manifest. Instead, loading is handled through [ledgerblue](https://pypi.org/project/ledgerblue/): the build generates an APDU install script and, when `--load` (or `-l`) is passed, runs it on the connected device with `python3 -m ledgerblue.runScript --targetId <id> --fileName <script> --apdu --scp`.

:warning: Loading must be performed **out of the Docker container** (it needs USB access to the device).

* Install `ledgerblue`:
```shell
pip3 install ledgerblue
```
* Load on device, e.g. for Flex:
```bash
python3 -m ledgerblue.runScript --targetId <id> --fileName target/flex/release/app-boilerplate-rust.apdu --apdu --scp
```

ℹ️ Your device must be connected, unlocked and the screen showing the dashboard (not inside an application).

#### About the device target ID

ledgerblue needs the device's `targetId`. 

If you call ledgerblue manually, note that its `--targetId` defaults to `0x31100002` (Nano S) — wrong for every other device, and it is **not** auto-detected from the connected device. The cleanest option is to let ledgerblue read the id straight from the ELF with `--elfFile`, which overrides `--targetId`:
```bash
python3 -m ledgerblue.runScript --elfFile target/flex/release/app-boilerplate-rust --fileName <install_script> --apdu --scp
```

If you instead need the raw target ID value (e.g. for a CI script), the [`tools/get_target_id.py`](tools/get_target_id.py) helper extracts it from the `ledger.target_id` ELF section:
```bash
python3 tools/get_target_id.py target/flex/release/app-boilerplate-rust   # -> 0x33300004
```

## Continuous Integration
The following workflows are executed in [GitHub Actions](https://github.com/features/actions) :

* Ledger guidelines enforcer which verifies that an app is compliant with Ledger guidelines. The successful completion of this reusable workflow is a mandatory step for an app to be available on the Ledger application store. More information on the guidelines can be found in the repository [ledger-app-workflow](https://github.com/LedgerHQ/ledger-app-workflows)
* Compilation of the application for all supported devices in the [ledger-app-builder](https://github.com/LedgerHQ/ledger-app-builder) docker image
* End-to-end tests with the [Speculos](https://github.com/LedgerHQ/speculos) emulator and [ragger](https://github.com/LedgerHQ/ragger) (see [tests/](tests/))
* Various lint checks :
  * Source code lint checks with `cargo fmt`
  * Python functional test code lint checks with `pylint` and `mypy`

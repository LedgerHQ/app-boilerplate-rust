# Quantus Ledger App

![Rule enforcer](https://github.com/LedgerHQ/ledger-app-rs/actions/workflows/guidelines_enforcer.yml/badge.svg) ![Build and tests](https://github.com/LedgerHQ/ledger-app-rs/actions/workflows/build_and_functional_tests.yml/badge.svg)

This is a quantus ledger application written in Rust which can be forked to start a new project for the Ledger Nano X, S+, Stax and Flex devices.

:warning: Nano S is not supported

* Implements standard features (display address, transaction signature...),
* Has functional tests using [Ragger](https://github.com/Quantus-Network/ragger),
* Has CI workflows mandatory for app deployment in the Ledger store.

### Links

* 📚 [Developer's documentation](https://developers.ledger.com/)<br/>
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
* Open a terminal and clone `ledger-app-rs` with `git clone git@github.com:Quantus-Network/ledger-app-rs.git`.
* Open the `ledger-app-rs` folder with VSCode.
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
sudo docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
```
* Make sure you have an X11 server running :
  * On Ubuntu Linux, it should be running by default.
  * On macOS, install and launch [XQuartz](https://www.xquartz.org/) (make sure to go to XQuartz > Preferences > Security and check "Allow client connections").
  * On Windows, install and launch [VcXsrv](https://sourceforge.net/projects/vcxsrv/) (make sure to configure it to disable access control).
* You can then enter into this development environment by executing the following command from the directory of the application (`git` repository):
  * Linux (Ubuntu): 
  ```shell
  sudo docker run --rm -ti --privileged -v "/dev/bus/usb:/dev/bus/usb" -v "$(realpath .):/app" --publish 5001:5001 --publish 9999:9999 -e DISPLAY=$DISPLAY -v '/tmp/.X11-unix:/tmp/.X11-unix' ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
  ```
  * macOS:
  ```shell
  sudo docker run  --rm -ti --privileged -v "$(pwd -P):/app" --publish 5001:5001 --publish 9999:9999 -e DISPLAY='host.docker.internal:0' -v '/tmp/.X11-unix:/tmp/.X11-unix' ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
  ```
  * Windows (with PowerShell):
  ```shell
  docker run --rm -ti --privileged -v "$(Get-Location):/app" -e DISPLAY='host.docker.internal:0' --publish 5001:5001 --publish 9999:9999 ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest
  ```

The application's code will be available from inside the docker container, you can proceed to the following compilation steps to build your app.

### Building

You can build the boilerplate with the following command executed in the root directory of the app.
```bash
cargo ledger build nanox
```
This command will build the app for the Nano X, but you can use any supported device (`nanox`, `nanosplus`, `stax`, `flex`)

### Testing
#### Ragger functional tests
This boilerplate app comes with functional tests implemented with Ledger's [Ragger](https://github.com/LedgerHQ/ragger) test framework.

* Install the tests requirements
```bash
pip install -r tests/requirements.txt 
```
* Run the functional tests :

```shell
pytest tests/ --tb=short -v --device {nanosp | nanox | stax | flex}
```
#### Emulator
You can also run the app directly on the [Speculos emulator](https://github.com/LedgerHQ/speculos) from the Docker container
#### Nano S+ or X
```bash
speculos --apdu-port 9999 --api-port 5001 --display headless --model nanosp target/nanosplus/release/ledger-app-rs
```
:warning: UI is displayed on `localhost:5001`
#### Stax or Flex
```bash
speculos --apdu-port 9999 --api-port 5001 --model stax target/stax/release/ledger-app-rs
```
:warning: UI is displayed by your X server

You can then send APDU using `ledgercomm` (`pip install ledgercomm`):
```
ledgercomm-send file test.apdu
```
### Loading on device
:warning: Loading the built application on a device shall be performed out of the Docker container, by using [ledgerctl](https://github.com/LedgerHQ/ledgerctl):
```shell
pip3 install ledgerwallet
````
ℹ️ Your device must be connected, unlocked and the screen showing the dashboard (not inside an application). 

For instance, for Flex:
```bash
ledgerctl install -f target/flex/release/app_flex.json
```

## Continuous Integration
The following workflows are executed in [GitHub Actions](https://github.com/features/actions) :

* Ledger guidelines enforcer which verifies that an app is compliant with Ledger guidelines. The successful completion of this reusable workflow is a mandatory step for an app to be available on the Ledger application store. More information on the guidelines can be found in the repository [ledger-app-workflow](https://github.com/LedgerHQ/ledger-app-workflows)
* Compilation of the application for all supported devices in the [ledger-app-builder](https://github.com/LedgerHQ/ledger-app-builder) docker image
* End-to-end tests with the [Speculos](https://github.com/LedgerHQ/speculos) emulator and [ragger](https://github.com/LedgerHQ/ragger) (see [tests/](tests/))
* Various lint checks :
  * Source code lint checks with `cargo fmt`
  * Python functional test code lint checks with `pylint` and `mypy`
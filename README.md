HealthPi
========

Utility to fetch and read data from smart health devices.

Supported devices
-----------------

Currently the following devices are supported:

* Soehnle Shape Sense Connect 200 (scale)
* Soehnle Systo Monitor Connect 400 (blood pressure monitor)
* Ascensia Contour Plus Elite (glucometer)

Running the application
-----------------------

The application uses `bluez_async` for Bluetooth LE connectivity, which
only works on Linux machines. It has been tested on Raspberry Pi 4.

The entire thing is written in Rust, which means you will have to install
cargo to compile and run the program. You will also need `libdbus`
and `pkg-config` installed, which you can do on Ubuntu with:

```
sudo apt install libdbus-1-dev pkg-config
```

After that simply run:

```
cargo run --release
```

in the project root directory in order to run the program.

### Device setup

Currently HealthPi does not support pairing devices. In order to set up a device
to work with HealthPi, you need to create a `devices.csv` file, including one
device MAC address per file. If your device needs a special pairing procedure
(e.g. providing one-time code to establish an encrypted connection), you need
to perform it through an external tool, e.g. `bluetoothctl`. This only needs
to be done one per device.

### Database setup

HealthPi uses [sqlx-cli](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md) 
to manage database migrations. At this point, you need to run the migrations manually. 
sqlx-cli is included in the nix development shell, so you can set up the database by
running:

```
nix develop
sqlx database create
sqlx migrate run
```

If you do not want to use nix, you can install sqlx-cli with

```
cargo install sqlx-cli
```

Local development setup
-----------------------

### Using nix

#### Setup

1. Install nix by following instructions at https://nixos.org/download
2. Add the following line to /etc/nix/nix.conf:
```nix
experimental-features = nix-command flakes
```
3. (optional) Install [direnv](https://direnv.net) and [direnv VSCode extension](https://marketplace.visualstudio.com/items?itemName=mkhl.direnv).

#### Workflow

Use `nix develop` to enter development shell. From there you'll have access
to any tools and dependencies needed for development.

### Git hooks

This repository has git hooks prepared that check simple conditions that might
otherwise trip up the CI setup. We recommend that you use them. In order to set
them up, run the following command inside the repository:

```
git config core.hooksPath .githooks
```
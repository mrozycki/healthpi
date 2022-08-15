WagaPi
======

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

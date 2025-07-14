# ut61e+ datalogger

Thanks to the excellent work by ljakob with their python usb hid reverse engineering project,
(ljakob/unit_ut61eplus)[https://github.com/ljakob/unit_ut61eplus]. I used their code as a
reference as well as some packet sniffing myself to get a simple logger built. It uses udev
to find the USB logger and start a logging session, the packets are then parsed to get the mode,
range and other info that might be on screen

## install

I've included a simple udev rule file that you can copy to `/etc/udev/rules.d/` and then run

```fish
sudo cp 99-ut61e-plus.rules /etc/udev/rules.d/
sudo udevadm control --reload
sudo udevadm trigger
```

If your device is still not detected you should check the output of `lsusb` for the USB VID and
PID of your meter, and open an issue.

Then won't need `sudo` to run the logger.

In addition to the dependencies in `Cargo.toml`, this needs `systemd-devel` to build.

To build the project, run
```
cargo build --release
```

## Usage

You can run it with `--csv` for a simple output and with nothing for some pretty logging.

```
./target/release/ut61e_plus --csv
```

## Notes

It DOES NOT do the following which ljakob's code does.
- Let you control the meter.
- Support other meters than the ut61e+.
- MQTT functionality (yet...).

Things I still want to do.
- `--timestamp` option
- `--ascii-only` flag to disable the cool unicode characters (like 𜰏)

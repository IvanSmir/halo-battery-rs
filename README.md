# halo-battery-rs

Battery levels of wireless devices in the Windows system tray, without vendor
software. A Rust port of [HaloBattery](https://github.com/HeyOkay/HaloBattery).

Each device gets its own tray icon: a ring that fills clockwise with the
battery level and the device pictogram in the middle.

## Supported devices

| Device | How |
| --- | --- |
| Logitech mice and keyboards (Lightspeed / Unifying, e.g. PRO X Superlight 2) | HID++ 2.0, features `0x1004` / `0x1000` / `0x1001` |
| Xbox-compatible controllers (e.g. GameSir G7 Pro on its 2.4 GHz receiver) | Windows.Gaming.Input battery report |

## Build

```
cargo build --release
```

The binary is `target/release/halo-battery.exe`.

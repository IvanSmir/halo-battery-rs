# halo-battery-rs

Battery levels of wireless devices in the Windows system tray, without vendor
software. A Rust port of [HaloBattery](https://github.com/HeyOkay/HaloBattery):
a single ~800 KB executable that idles at a few MB of memory.

Each device gets its own tray icon: a ring that fills clockwise with the
battery level and the device pictogram in the middle. The ring takes the
taskbar colour, turns amber close to the low threshold, red at or below it and
breathes green while charging. A device that is asleep keeps its last reading,
translucent, for five minutes. Hover an icon for the exact percentage.

## Supported devices

| Device | How |
| --- | --- |
| Logitech mice and keyboards (Lightspeed / Unifying, e.g. PRO X Superlight 2) | HID++ 2.0, features `0x1004` / `0x1000` / `0x1001` |
| Xbox-compatible controllers (e.g. GameSir G7 Pro on its 2.4 GHz receiver) | Windows.Gaming.Input battery report |

Every query is a read; no device setting is ever changed.

## Usage

Run `halo-battery.exe`. Right-click any of its icons for:

- **Actualizar ahora** - read all batteries now
- **Intervalo de lectura** - 15 s to 5 min (default 1 min)
- **Aviso de batería baja** - 10 % to 30 % (default 20 %); a notification is shown once per discharge
- **Iniciar con Windows** - adds or removes the entry under `HKCU\...\CurrentVersion\Run`

Settings live in `%APPDATA%\halo-battery-rs\config.json`.

`halo-battery.exe --list` prints what each provider sees, useful when a device
does not show up.

## Build

```
cargo build --release
```

The binary is `target/release/halo-battery.exe`.

## Development

```
cargo test                          # integration tests in tests/
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run --example icon_preview    # every icon variant -> target/icon-preview.png
```

CI runs the same checks on Windows for every push.

## Architecture

A library crate holds everything; `src/main.rs` only picks between the tray
app and `--list`. Pure logic is kept apart from I/O so it can be tested
without hardware.

```
src/
  device.rs            data model: DeviceStatus, Kind
  providers/           hardware -> DeviceStatus
    mod.rs             Provider trait and the registry of providers
    last_seen.rs       keeps asleep devices, greyed out, for a while
    logitech/
      protocol.rs      HID++ 2.0 messages and decoding (pure)
      channel.rs       request/response I/O over hidapi
      mod.rs           receiver discovery and slot tracking
    gamepad/
      mapping.rs       names and capacity -> percent (pure)
      mod.rs           Windows.Gaming.Input reads
  icon/                IconState -> RGBA pixels (pure)
    palette.rs         colours and the arc colour rule
    shapes.rs          drawing primitives
    pictograms.rs      device silhouettes
  app/                 the tray application
    poller.rs          background thread that reads the providers
    view.rs            DeviceStatus -> icon state and tooltip (pure)
    alerts.rs          when to warn about a low battery (pure)
    menu.rs            context menu -> Action values
    tray.rs            keeps the system tray in sync with the views
    mod.rs             wiring and the Win32 message loop
  platform/            thin Windows wrappers: theme, autostart, toast, ...
  config.rs            settings file, validated on load
  cli.rs               --list diagnostics
tests/                 one integration test crate per module
examples/icon_preview.rs
```

To support a new device family, add a module under `src/providers/` that
implements `Provider` and register it in `providers::all()`.

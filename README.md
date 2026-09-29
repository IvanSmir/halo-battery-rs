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
| Bluetooth headsets, keyboards, mice and controllers whose level Windows shows in Settings | `DEVPKEY_Bluetooth_Battery` through SetupAPI, connection state through WinRT |

Every query is a read; no device setting is ever changed.

## Usage

Run `halo-battery.exe`. Click any of its icons (or pick **Configuración…**
from the right-click menu) to open the settings window:

- **Dispositivos** - every detected device with its live level; rename it,
  hide it from the tray or silence its notifications
- **Notificaciones** - master switch, low-battery threshold (once per
  discharge), full-charge alert and sound
- **Apariencia** - ring colour, charging animation, pictogram
- **General** - poll interval, start with Windows and **Exportar diagnóstico**

The right-click menu also has **Actualizar ahora** and **Salir**.

`halo-battery.exe --list` prints what each provider sees, useful when a device
does not show up.

### Supporting a new device

Ask for the exact model, how it connects (dongle, Bluetooth or cable, since
each mode is a different protocol), whether its official software shows the
battery, and a diagnostics report: **General → Exportar diagnóstico** in the
settings window, or `halo-battery.exe --diagnose [file]`. The report is written
to the Desktop and lists what the providers recognise, every HID interface with
its report descriptor (flagging standard battery usages such as Generic Device
Controls / Battery Strength), and the Bluetooth devices with the battery
Windows knows. It only reads: nothing is ever sent to a device.

Then look the VID/PID up in projects that document protocols (Solaar,
HeadsetControl, OpenRGB, libratbag). If none does and the official software
shows the battery, capture its traffic with USBPcap + Wireshark while the level
updates. Never send commands to an unknown device by trial and error: vendor
channels mix queries with settings.

### How the two programs talk

The tray (`halo-battery.exe`, ~3 MB of memory) and the settings window
(`halo-settings.exe`, Tauri, only running while open) share two files in
`%APPDATA%\halo-battery-rs`:

- `devices.json` - written by the tray after every reading; the window shows it
  and never talks to the hardware itself
- `config.json` - written by the window on every change; the tray applies it
  as soon as it changes

Both are written atomically (temporary file + rename).

## Build

```
cargo build --release                      # the tray
cargo build --release -p halo-settings     # the settings window (Tauri)
```

Both binaries land in `target/release/` and must stay side by side.

`pwsh scripts/release.ps1` builds the installer and a portable zip into
`dist/` (needs Rust and Node.js; the Tauri CLI runs through npx). The
**Build installer** workflow does the same on GitHub on demand.

## Releases

Versions and the changelog are automated with
[release-please](https://github.com/googleapis/release-please), driven by
[Conventional Commits](https://www.conventionalcommits.org):

| Commit | Effect on the next release |
| --- | --- |
| `fix: ...` | patch: 0.1.0 -> 0.1.1 |
| `feat: ...` | minor: 0.1.0 -> 0.2.0 |
| `feat!: ...` or a `BREAKING CHANGE:` footer | major |
| `docs:`, `refactor:`, `test:`, `build:`, `ci:`, `chore:` | none, hidden from the changelog |

On every push to `main`, release-please keeps a release pull request up to
date with the version bump (both `Cargo.toml` files and `Cargo.lock`) and the
new `CHANGELOG.md` entries. Merging it tags `vX.Y.Z`, publishes the GitHub
release and attaches the installer and the portable zip. The app version is
only in the `Cargo.toml` files; Tauri reads it from there.

## Development

```
cargo test                                      # integration tests in tests/
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run --example icon_preview                # every icon variant -> target/icon-preview.png
```

CI runs the same checks on Windows for every push.

## Architecture

A cargo workspace: the root crate is the library plus the tray binary
(`src/main.rs` only picks between the tray app and `--list`); `settings/` is
the Tauri window. Pure logic is kept apart from I/O so it can be tested
without hardware.

```
src/
  device.rs            data model: DeviceStatus, Kind
  providers/           hardware -> DeviceStatus
    mod.rs             Provider trait and the registry of providers
    last_seen.rs       keeps asleep devices, greyed out, for a while
    logitech/
      protocol.rs      HID++ 2.0 messages and decoding (pure)
      transport.rs     request/response contract and multi-request reads
      channel.rs       the transport over hidapi
      slot.rs          reads one receiver slot, one request once known
      mod.rs           receiver discovery
    gamepad/
      mapping.rs       names and capacity -> percent (pure)
      mod.rs           Windows.Gaming.Input reads
  icon/                IconState -> RGBA pixels (pure)
    palette.rs         colours and the arc colour rule
    shapes.rs          drawing primitives
    pictograms.rs      device silhouettes
  app/                 the tray application
    poller.rs          background thread that reads the providers
    view.rs            devices + settings -> icon states and tooltips (pure)
    alerts.rs          when to notify (pure)
    menu.rs            context menu -> Action values
    tray.rs            keeps the system tray in sync with the views
    mod.rs             wiring, shared files and the Win32 message loop
  platform/            thin Windows wrappers: theme, autostart, toast, launch, ...
  config.rs            settings file, validated on load
  snapshot.rs          the last readings, published for the window
  storage.rs           data directory, atomic JSON writes, file watching
  cli.rs               --list diagnostics
settings/              the settings window
  src/main.rs          commands over the shared files, pushes live updates
  ui/                  HTML, CSS and JS (no bundler)
tests/                 one integration test crate per module
examples/icon_preview.rs
```

To support a new device family, add a module under `src/providers/` that
implements `Provider` and register it in `providers::all()`.

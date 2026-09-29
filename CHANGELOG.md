# Changelog

## [0.2.0](https://github.com/IvanSmir/halo-battery-rs/compare/v0.1.0...v0.2.0) (2026-09-29)


### Features

* announce when an update is available ([e0ff810](https://github.com/IvanSmir/halo-battery-rs/commit/e0ff81001bca682c876271f4e8174f2678af29bc))
* export a diagnostics report to support new devices ([b9f8b9e](https://github.com/IvanSmir/halo-battery-rs/commit/b9f8b9e3d729e16c55c3517a430333544327a69e))
* show the battery of Bluetooth devices Windows knows ([e4fab8a](https://github.com/IvanSmir/halo-battery-rs/commit/e4fab8aded501722d070921f8915f12fe787128a))


### Bug Fixes

* write the diagnostics report in English like the rest of --list ([2602efb](https://github.com/IvanSmir/halo-battery-rs/commit/2602efbbc53e55e10d93fe71545498c1fe81f7a7))

## 0.1.0 (2026-09-29)

First release.

### Features

* Battery levels in the Windows tray, one icon per device: a ring that fills
  with the level around the device pictogram, amber near the low threshold,
  red below it and breathing green while charging
* Logitech mice and keyboards over HID++ 2.0 (Lightspeed / Unifying
  receivers), e.g. PRO X Superlight 2
* Xbox-compatible controllers through Windows.Gaming.Input, e.g. GameSir G7 Pro
* Settings window: rename, hide or silence each device; low-battery and
  full-charge notifications with or without sound; ring colour, charging
  animation and pictogram; poll interval; start with Windows
* Windows installer (per user, no admin rights) and portable zip

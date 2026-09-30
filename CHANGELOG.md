# Changelog

## [0.3.0](https://github.com/IvanSmir/halo-battery-rs/compare/v0.2.0...v0.3.0) (2026-09-30)


### Features

* add a structured error type for the settings commands ([3e9eaa0](https://github.com/IvanSmir/halo-battery-rs/commit/3e9eaa02de2840c7d9284f96b4ff5d747fcad84d))
* choose the ring colour per device ([bdf0e1b](https://github.com/IvanSmir/halo-battery-rs/commit/bdf0e1b1592ea28a9397cb2df7a74c40d8b3df37))
* pick a ring colour on each device card ([8b3f20c](https://github.com/IvanSmir/halo-battery-rs/commit/8b3f20c96fbaa3e6a31230af9b03ee5f79b219f8))
* show backend errors in the settings window ([57444da](https://github.com/IvanSmir/halo-battery-rs/commit/57444dab2e3bee69ac5459b1c50f8723415300b2))
* tell a corrupt config apart from a missing one ([ba80a2a](https://github.com/IvanSmir/halo-battery-rs/commit/ba80a2acdf419381d9cf9d84af59b7ba418d5757))


### Bug Fixes

* keep the tray icon pinned by giving each device a stable identity ([cf3a085](https://github.com/IvanSmir/halo-battery-rs/commit/cf3a0853b482a8abe4aa45f33b27fa88b85ab0db))
* report autostart registry failures instead of dropping them ([c7df6c0](https://github.com/IvanSmir/halo-battery-rs/commit/c7df6c07cec3a833d59e48736aaabffc26c557c9))

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

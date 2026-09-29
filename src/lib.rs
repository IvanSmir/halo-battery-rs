//! Battery levels of wireless devices in the Windows system tray.
//!
//! The crate is split in layers:
//! - [`device`]: the data model shared by everything else
//! - [`providers`]: talk to the hardware and produce [`device::DeviceStatus`]
//! - [`icon`]: draw the tray icon for a status (pure, no I/O)
//! - [`platform`]: thin wrappers over the Windows APIs the app needs
//! - [`config`]: user settings on disk
//! - [`snapshot`]: the last device readings, published for the settings window
//! - [`storage`]: where the files live and how they are written
//! - [`app`]: the tray application that wires it all together
//! - [`cli`]: the `--list` and `--diagnose` commands
//! - [`diagnose`]: the diagnostics report used to support new devices

pub mod app;
pub mod cli;
pub mod config;
pub mod device;
pub mod diagnose;
pub mod icon;
pub mod platform;
pub mod providers;
pub mod snapshot;
pub mod storage;

//! Re-Link: keeps browser connections on the FortiClient VPN split tunnel.
//! The library holds everything testable; `main.rs` wires the app.

/// The Windows AppUserModelID and notification identity.
pub const APP_ID: &str = "ReLink.App";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod bookmarks;
pub mod config;
pub mod diagnose;
pub mod engine;
pub mod helper;
pub mod i18n;
pub mod iconart;
pub mod monitor;
pub mod net;
pub mod platform;
pub mod tray;
pub mod ui;
pub mod update;

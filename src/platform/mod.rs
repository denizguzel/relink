//! Small OS integrations, one module each: autostart, opening files,
//! local time, window geometry, `curl`, notifications, the session lock and the VPN
//! client.

mod curl;
mod desktop;
mod geometry;
mod install_entry;
mod notify;
#[cfg(windows)]
mod own_window;
mod placement;
mod process;
mod session;
mod startup;
mod time;
mod titlebar;
mod vpn_client;
mod window;

pub use curl::curl;
pub use desktop::{attach_console, open_in_editor, open_url};
pub use geometry::{MIN_SIZE, default_window_size, pick_size, plausible_size};
pub use install_entry::sync_install_entry;
pub use notify::{notify, on_notification_click};
pub use placement::{place_window, screen_work_area};
pub use process::end_previous;
pub use session::session_locked;
pub use startup::{HIDDEN_ARG, set_startup, startup_enabled};
pub use time::{local_parts, local_time};
pub use titlebar::{TitleBar, match_title_bar};
pub use vpn_client::{open_vpn_client, vpn_client_available};
pub use window::{window_exists, window_geometry};

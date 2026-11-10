//! Small OS integrations, one module each: autostart, opening files,
//! local time, window geometry, notifications and the VPN client.

mod desktop;
mod geometry;
mod notify;
#[cfg(windows)]
mod own_window;
mod placement;
mod process;
mod startup;
mod time;
mod titlebar;
mod vpn_client;
mod window;

pub use desktop::{attach_console, open_in_editor, open_url};
pub use geometry::{default_window_size, pick_size};
pub use notify::{notify, on_notification_click};
pub use placement::{place_window, screen_work_area};
pub use process::wait_for_exit;
pub use startup::{HIDDEN_ARG, set_startup, startup_enabled};
pub use time::{local_parts, local_time};
pub use titlebar::{TitleBar, match_title_bar};
pub use vpn_client::{open_vpn_client, vpn_client_available};
pub use window::window_geometry;

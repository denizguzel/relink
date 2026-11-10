//! Small OS integrations, one module each: autostart, opening files,
//! local time, window geometry and notifications.

mod desktop;
mod geometry;
mod notify;
mod process;
mod startup;
mod time;
mod window;

pub use desktop::{attach_console, open_in_editor, open_url};
pub use geometry::{default_window_size, pick_size};
pub use notify::{notify, on_notification_click};
pub use process::wait_for_exit;
pub use startup::{HIDDEN_ARG, set_startup, startup_enabled};
pub use time::{local_parts, local_time};
pub use window::{move_window, screen_work_area, window_geometry};

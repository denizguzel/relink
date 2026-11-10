//! The Re-Link window: a sidebar of pages over the monitor's latest report.
//! State lives in `state`; the monitor thread pushes reports in through
//! `AppHandle::update`, and handlers send commands back through `Link`.

mod close;
pub mod demo;
mod diagnostics;
pub mod icons;
mod input;
mod pages;
mod parts;
pub mod pause;
mod shell;
mod sidebar;
mod state;
mod status;
#[cfg(test)]
mod tests;
mod theme;
mod tick;
mod welcome;

pub use close::{HAS_TRAY, on_close};
pub use diagnostics::{clean_name, diagnostics};
pub use input::on_key;
pub use shell::view;
pub use state::*;
pub use status::{short_status, status_text};
pub use tick::poll;
pub use welcome::wanted as welcome_wanted;

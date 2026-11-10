//! The five pages, one module each.

mod activity;
mod connections;
pub(super) mod controls;
mod hosts;
mod overview;
mod pills;
mod settings;

pub use activity::activity;
pub use connections::connections;
pub use hosts::hosts;
pub use overview::overview;
pub use settings::settings;

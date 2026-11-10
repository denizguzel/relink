//! Closing the window: the first time, say that Re-Link keeps running in the tray.

use std::sync::atomic::{AtomicBool, Ordering};

use rune::Tx;

use super::Ui;
use crate::config::Config;
use crate::t;

/// Whether the app has a tray icon (set once at startup); without one, closing
/// the window quits and there is nothing to explain.
pub static HAS_TRAY: AtomicBool = AtomicBool::new(false);

/// The notice is shown once, and only when notifications are not switched off.
pub fn should_show_tray_notice(cfg: &Config, has_tray: bool) -> bool {
    has_tray && !cfg.tray_notice_shown && cfg.notifications != "off"
}

pub fn on_close(s: &mut Ui, _tx: &mut Tx) {
    if should_show_tray_notice(&s.cfg, HAS_TRAY.load(Ordering::Relaxed)) {
        s.change(|c| c.tray_notice_shown = true);
        crate::platform::notify(
            "relink-tray-notice",
            &t!("tray_notice_title"),
            &t!("tray_notice_body"),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(shown: bool, level: &str) -> Config {
        Config {
            tray_notice_shown: shown,
            notifications: level.into(),
            ..Config::default()
        }
    }

    #[test]
    fn shown_once_when_there_is_a_tray() {
        assert!(should_show_tray_notice(&cfg(false, "problems"), true));
        assert!(!should_show_tray_notice(&cfg(true, "problems"), true));
    }

    #[test]
    fn not_without_a_tray_or_with_notifications_off() {
        assert!(!should_show_tray_notice(&cfg(false, "problems"), false));
        assert!(!should_show_tray_notice(&cfg(false, "off"), true));
        assert!(should_show_tray_notice(&cfg(false, "all"), true));
    }
}

//! Closing the window: the first time, say that Re-Link keeps running in the tray.

use std::sync::atomic::{AtomicBool, Ordering};

use rune::Tx;

use super::Ui;
use crate::config::Config;
use crate::t;

/// Whether the app has a tray icon (set once at startup); without one, closing
/// the window quits and there is nothing to explain.
pub static HAS_TRAY: AtomicBool = AtomicBool::new(false);

/// Whether closing the window quits the app: the user asked for it. (Without
/// a tray the app quits anyway, so this is only about a running tray.)
pub fn closing_quits(cfg: &Config, has_tray: bool) -> bool {
    has_tray && !cfg.close_to_tray
}

/// The notice is shown once, only when closing really keeps Re-Link running,
/// and only when notifications are not switched off.
pub fn should_show_tray_notice(cfg: &Config, has_tray: bool) -> bool {
    has_tray && cfg.close_to_tray && !cfg.tray_notice_shown && cfg.notifications != "off"
}

pub fn on_close(s: &mut Ui, tx: &mut Tx) {
    if closing_quits(&s.cfg, HAS_TRAY.load(Ordering::Relaxed)) {
        tx.quit();
        return;
    }
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
    fn closing_quits_only_when_asked_and_a_tray_would_keep_it_alive() {
        let mut c = Config::default();
        assert!(!closing_quits(&c, true));
        c.close_to_tray = false;
        assert!(closing_quits(&c, true));
        assert!(
            !closing_quits(&c, false),
            "no tray: the window system quits it"
        );
        assert!(!should_show_tray_notice(&c, true), "nothing keeps running");
    }

    #[test]
    fn not_without_a_tray_or_with_notifications_off() {
        assert!(!should_show_tray_notice(&cfg(false, "problems"), false));
        assert!(!should_show_tray_notice(&cfg(false, "off"), true));
        assert!(should_show_tray_notice(&cfg(false, "all"), true));
    }
}

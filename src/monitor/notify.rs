//! Desktop notifications, filtered by the level chosen in the settings.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::t;

/// The notification level from the settings, shared with `notify`.
static NOTIFY_LEVEL: Mutex<String> = Mutex::new(String::new());
/// Notifications are paused (the pause button or the tray menu).
static PAUSED: AtomicBool = AtomicBool::new(false);

/// Whether a notification of `kind` passes the level: "off" shows none,
/// "problems" only stuck connections and missing routes, anything else all.
/// While paused nothing is shown.
fn allowed(level: &str, kind: &str, paused: bool) -> bool {
    if paused {
        return false;
    }
    match level {
        "off" => false,
        "problems" => matches!(kind, "stuck" | "missing"),
        _ => true,
    }
}

/// Posts a notification. `action` tells the click handler what to do.
pub(super) fn notify(kind: &str, title: &str, body: &str, action: Option<&str>) {
    if !allowed(
        &NOTIFY_LEVEL.lock().unwrap(),
        kind,
        PAUSED.load(Ordering::Relaxed),
    ) {
        return;
    }
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let label = match action {
        Some("fix") => t!("fix_now"),
        _ => t!("open_window"),
    };
    crate::platform::notify(
        &format!("relink-{kind}-{n}"),
        title,
        body,
        action.map(|a| (a, label.as_str())),
    );
}

/// Mutes or unmutes every notification.
pub(super) fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}

/// Takes the level from the current settings.
pub(super) fn set_level(level: &str) {
    *NOTIFY_LEVEL.lock().unwrap() = level.to_string();
}

#[cfg(test)]
mod tests {
    use super::allowed;

    #[test]
    fn levels_filter_notification_kinds() {
        assert!(allowed("all", "fixed", false) && allowed("all", "route", false));
        assert!(allowed("problems", "stuck", false) && allowed("problems", "missing", false));
        assert!(!allowed("problems", "fixed", false) && !allowed("problems", "route", false));
        assert!(!allowed("off", "stuck", false));
    }

    #[test]
    fn a_pause_silences_every_level() {
        for level in ["all", "problems", "off"] {
            for kind in ["stuck", "missing", "fixed", "route"] {
                assert!(!allowed(level, kind, true), "{level}/{kind}");
            }
        }
    }

    #[test]
    fn an_unknown_level_shows_everything() {
        assert!(allowed("garbage", "fixed", false));
    }
}

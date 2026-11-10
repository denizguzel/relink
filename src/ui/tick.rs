//! Work done on the window's once-a-second tick.

use rune::Tx;

use super::Ui;

/// Puts the window where it belongs once it exists (back where it was, kept
/// on screen; or centred on the active monitor the first time), then saves its
/// geometry whenever it has settled on a new one.
fn remember_window(s: &mut Ui) {
    let Some(path) = s.link.as_ref().map(|l| l.window_path.clone()) else {
        return;
    };
    let Some(now) = crate::platform::window_geometry() else {
        return;
    };
    if !s.window_restored {
        s.window_restored = true;
        let saved = crate::config::WindowGeom::load(&path).map(|g| (g.x, g.y));
        crate::platform::place_window(saved);
        return;
    }
    let settled = s.window_seen == Some(now);
    s.window_seen = Some(now);
    if settled && s.window_saved != Some(now) {
        s.window_saved = Some(now);
        let _ = now.save(&path);
    }
}

/// Re-reads per-second state; returns whether the view must rebuild.
pub fn poll(s: &mut Ui, _clock: f64, tx: &mut Tx) -> bool {
    // Until the window has been placed, look again quickly (for the first few
    // seconds) so it does not sit in the wrong spot for long.
    s.place_polls = s.place_polls.saturating_add(1);
    let wait = if s.window_restored || s.place_polls > 50 {
        1000
    } else {
        100
    };
    tx.wake_after(std::time::Duration::from_millis(wait));
    remember_window(s);
    let now = crate::config::now_unix();
    let locale = crate::i18n::current();
    let changed = now != s.now || locale != s.locale;
    s.now = now;
    s.locale = locale;
    changed
}

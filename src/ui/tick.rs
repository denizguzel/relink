//! Work done on the window's once-a-second tick.

use rune::Tx;

use super::Ui;

/// Puts the window back where it was, once, then saves its geometry whenever
/// it has settled on a new one.
fn remember_window(s: &mut Ui) {
    let Some(path) = s.link.as_ref().map(|l| l.window_path.clone()) else {
        return;
    };
    let Some(now) = crate::platform::window_geometry() else {
        return;
    };
    if !s.window_restored {
        s.window_restored = true;
        if let Some(saved) = crate::config::WindowGeom::load(&path) {
            crate::platform::move_window(saved.x, saved.y);
            return;
        }
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
    tx.wake_after(std::time::Duration::from_secs(1));
    remember_window(s);
    let now = crate::config::now_unix();
    let locale = crate::i18n::current();
    let changed = now != s.now || locale != s.locale;
    s.now = now;
    s.locale = locale;
    changed
}

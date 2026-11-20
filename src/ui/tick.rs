//! Work done on the window's once-a-second tick.

use rune::Tx;

use super::Ui;
use crate::config::WindowGeom;

/// What to do about the window's place on this tick.
#[derive(Debug, PartialEq)]
enum Step {
    /// Nothing to do yet: there is no window (closed to the tray, minimized).
    Wait,
    /// It (re)appeared at another size than the saved one: resize first, since
    /// keeping it on screen depends on its size.
    Resize((f32, f32)),
    /// Put it back where it was (or centre it the first time).
    Place,
    /// It is where the user put it: remember a geometry that settled.
    Track,
}

/// The saved geometry, unless its size cannot be the app window's (then its
/// position is not the window's either).
fn usable(saved: Option<WindowGeom>) -> Option<WindowGeom> {
    saved.filter(|g| crate::platform::plausible_size(g.width, g.height))
}

/// The next step for a window currently at `now` (None: none showing).
/// `restored`/`resized`: what was done since it last appeared.
fn step(
    now: Option<WindowGeom>,
    saved: Option<WindowGeom>,
    restored: bool,
    resized: bool,
    work: Option<(f32, f32)>,
) -> Step {
    let Some(now) = now else { return Step::Wait };
    if restored {
        return Step::Track;
    }
    if !resized && let Some(g) = saved {
        let want = crate::platform::pick_size(Some(g), work);
        if (want.0 - now.width).abs() > 2.0 || (want.1 - now.height).abs() > 2.0 {
            return Step::Resize(want);
        }
    }
    Step::Place
}

/// Puts the window where it belongs whenever it appears (back where it was and
/// at its size, kept on screen; or centred on the active monitor the first
/// time), then saves its geometry whenever it has settled on a new one. Closed
/// to the tray, Rune destroys the window and rebuilds it at its launch size,
/// so this runs again on every open.
fn remember_window(s: &mut Ui, tx: &mut Tx) {
    let Some(path) = s.link.as_ref().map(|l| l.window_path.clone()) else {
        return;
    };
    let now = crate::platform::window_geometry();
    if now.is_none() && !crate::platform::window_exists() {
        s.window_restored = false;
        s.window_resized = false;
        s.window_seen = None;
        return;
    }
    if let Some(now) = now {
        s.window_size = Some((now.width, now.height));
    }
    // Read only while placing: the file and the screen do not change under a
    // window that stays put.
    let (saved, work) = if s.window_restored {
        (None, None)
    } else {
        (
            usable(WindowGeom::load(&path)),
            crate::platform::screen_work_area(),
        )
    };
    match step(now, saved, s.window_restored, s.window_resized, work) {
        Step::Wait => {}
        Step::Resize(size) => {
            s.window_resized = true;
            tx.resize_window_instant(size);
        }
        Step::Place => {
            s.window_restored = true;
            s.window_resized = true;
            crate::platform::place_window(saved.map(|g| (g.x, g.y)));
        }
        Step::Track => {
            let Some(now) = now else { return };
            let settled = s.window_seen == Some(now);
            s.window_seen = Some(now);
            if settled
                && s.window_saved != Some(now)
                && crate::platform::plausible_size(now.width, now.height)
            {
                s.window_saved = Some(now);
                let _ = now.save(&path);
            }
        }
    }
}

/// Re-reads per-second state; returns whether the view must rebuild.
pub fn poll(s: &mut Ui, _clock: f64, tx: &mut Tx) -> bool {
    // Until the window has been placed, look again quickly (for the first few
    // seconds after it was opened) so it does not sit in the wrong spot for long.
    s.place_polls = s.place_polls.saturating_add(1);
    let wait = if s.window_restored || s.place_polls > 50 {
        1000
    } else {
        100
    };
    tx.wake_after(std::time::Duration::from_millis(wait));
    remember_window(s, tx);
    let now = crate::config::now_unix();
    let locale = crate::i18n::current();
    let changed = now != s.now || locale != s.locale;
    s.now = now;
    s.locale = locale;
    changed
}

/// Shows the window and brings it to the front. Rune rebuilds a window that was
/// closed to the tray without activating it on Windows, so focus is asked for
/// separately; the quick polling then puts it back at its size and place.
pub fn bring_forward(handle: &rune::AppHandle<Ui>) {
    handle.update(|s, _| s.place_polls = 0);
    handle.open();
    handle.focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(width: f32, height: f32) -> Option<WindowGeom> {
        Some(WindowGeom {
            x: 10,
            y: 20,
            width,
            height,
        })
    }

    #[test]
    fn nothing_happens_without_a_window() {
        assert_eq!(
            step(None, geom(1200.0, 800.0), false, false, None),
            Step::Wait
        );
    }

    #[test]
    fn a_window_rebuilt_at_its_launch_size_is_resized_then_placed() {
        let saved = geom(1200.0, 800.0);
        assert_eq!(
            step(geom(1000.0, 680.0), saved, false, false, None),
            Step::Resize((1200.0, 800.0))
        );
        assert_eq!(
            step(geom(1200.0, 800.0), saved, false, true, None),
            Step::Place
        );
        assert_eq!(
            step(geom(1200.0, 800.0), saved, true, true, None),
            Step::Track
        );
    }

    #[test]
    fn a_resize_that_did_not_take_is_not_retried() {
        let saved = geom(1200.0, 800.0);
        assert_eq!(
            step(geom(1000.0, 680.0), saved, false, true, None),
            Step::Place
        );
    }

    #[test]
    fn the_right_size_or_no_saved_one_goes_straight_to_placing() {
        assert_eq!(
            step(geom(1200.0, 800.0), geom(1201.0, 799.0), false, false, None),
            Step::Place
        );
        assert_eq!(
            step(geom(1000.0, 680.0), None, false, false, None),
            Step::Place
        );
    }

    #[test]
    fn a_geometry_saved_from_another_window_is_ignored() {
        // Earlier versions could save a 0×0 helper window instead of the app's.
        assert_eq!(usable(geom(0.0, 0.0)), None);
        assert_eq!(usable(geom(1200.0, 800.0)), geom(1200.0, 800.0));
    }
}

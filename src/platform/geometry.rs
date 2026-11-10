//! Window size and position decisions, free of any OS call so they can be tested.

use crate::config::WindowGeom;

/// The smallest window the layout works in (logical pixels).
pub const MIN_SIZE: (f32, f32) = (760.0, 520.0);

/// The window's first size: roomier than the 1000×680 floor on big screens,
/// never more than 92% of the usable area on small ones (laptops).
pub fn default_window_size(work: Option<(f32, f32)>) -> (f32, f32) {
    let Some((w, h)) = work else {
        return (1000.0, 680.0);
    };
    (
        (w * 0.66).clamp(1000.0, 1400.0).min(w * 0.92),
        (h * 0.78).clamp(680.0, 940.0).min(h * 0.92),
    )
}

/// The size to open with: the saved one (kept inside the screen and above the
/// minimum), else the default for the screen.
pub fn pick_size(saved: Option<WindowGeom>, work: Option<(f32, f32)>) -> (f32, f32) {
    match saved {
        Some(g) => (
            g.width
                .clamp(MIN_SIZE.0, work.map_or(f32::MAX, |w| w.0).max(MIN_SIZE.0)),
            g.height
                .clamp(MIN_SIZE.1, work.map_or(f32::MAX, |w| w.1).max(MIN_SIZE.1)),
        ),
        None => default_window_size(work),
    }
}

/// Whether a window whose top-left is `(x, y)` can still be reached on a
/// desktop `(left, top, width, height)`: its title bar must stay on screen,
/// say after a monitor was unplugged.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn reachable(x: i32, y: i32, desktop: (i32, i32, i32, i32)) -> bool {
    let (left, top, width, height) = desktop;
    x >= left - 200 && y >= top && x <= left + width - 200 && y <= top + height - 100
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(width: f32, height: f32) -> Option<WindowGeom> {
        Some(WindowGeom {
            x: 0,
            y: 0,
            width,
            height,
        })
    }

    #[test]
    fn default_size_follows_the_screen() {
        assert_eq!(default_window_size(None), (1000.0, 680.0));
        let (w, h) = default_window_size(Some((1920.0, 1040.0)));
        assert!(w > 1000.0 && h > 680.0 && w <= 1400.0 && h <= 940.0);
        let (w, h) = default_window_size(Some((1280.0, 680.0)));
        assert!(w <= 1280.0 * 0.92 && h <= 680.0 * 0.92);
        assert_eq!(default_window_size(Some((5120.0, 2800.0))), (1400.0, 940.0));
    }

    #[test]
    fn a_saved_size_is_used_but_kept_sane() {
        assert_eq!(
            pick_size(saved(1100.0, 700.0), Some((1920.0, 1040.0))),
            (1100.0, 700.0)
        );
        // Smaller than the layout allows.
        assert_eq!(pick_size(saved(100.0, 100.0), None), MIN_SIZE);
        // Larger than the screen (a monitor was swapped for a smaller one).
        assert_eq!(
            pick_size(saved(3000.0, 2000.0), Some((1366.0, 728.0))),
            (1366.0, 728.0)
        );
        // No saved size: the default.
        assert_eq!(pick_size(None, None), (1000.0, 680.0));
    }

    #[test]
    fn a_tiny_screen_never_pushes_below_the_minimum() {
        assert_eq!(
            pick_size(saved(900.0, 600.0), Some((500.0, 400.0))),
            MIN_SIZE
        );
    }

    #[test]
    fn the_title_bar_must_stay_on_a_screen() {
        let desktop = (0, 0, 1920, 1080);
        assert!(reachable(100, 100, desktop));
        assert!(
            reachable(-100, 0, desktop),
            "a little off the left edge is fine"
        );
        assert!(
            !reachable(-1000, 100, desktop),
            "a monitor on the left that is gone"
        );
        assert!(!reachable(100, 2000, desktop), "below every screen");
        assert!(!reachable(3000, 100, desktop));
        // A desktop that starts left of zero (a second monitor on the left).
        assert!(reachable(-1500, 50, (-1920, 0, 3840, 1080)));
    }
}

//! Window size and position decisions, free of any OS call so they can be tested.

use crate::config::WindowGeom;

/// The smallest window the layout works in (logical pixels).
pub const MIN_SIZE: (f32, f32) = (760.0, 520.0);

/// The window's first size: roomier than the 1000×680 floor on big screens,
/// never more than 92% of the usable area on small ones (laptops). `work` is
/// the usable area of the monitor the window opens on, in logical pixels.
pub fn default_window_size(work: Option<(f32, f32)>) -> (f32, f32) {
    let Some((w, h)) = work else {
        return (1000.0, 680.0);
    };
    (
        (w * 0.72).clamp(1000.0, 1600.0).min(w * 0.92),
        (h * 0.82).clamp(680.0, 1000.0).min(h * 0.92),
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

/// A rectangle in physical screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// The top-left that centres a `width`×`height` window in `work`. A window
/// larger than the area is pinned to its top-left corner instead.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn centered(work: Rect, width: i32, height: i32) -> (i32, i32) {
    (
        work.left + ((work.width() - width) / 2).max(0),
        work.top + ((work.height() - height) / 2).max(0),
    )
}

/// The top-left that brings the window `win` inside `work`: unchanged when it
/// already fits, shifted when it hangs over an edge (say behind the taskbar
/// or off a monitor that is gone), pinned to the top-left when it is larger
/// than the area.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn fitted(work: Rect, win: Rect) -> (i32, i32) {
    let place = |pos: i32, size: i32, lo: i32, hi: i32| {
        if size >= hi - lo {
            lo
        } else {
            pos.clamp(lo, hi - size)
        }
    };
    (
        place(win.left, win.width(), work.left, work.right),
        place(win.top, win.height(), work.top, work.bottom),
    )
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
        assert!(w > 1000.0 && h > 680.0 && w <= 1600.0 && h <= 1000.0);
        let (w, h) = default_window_size(Some((1280.0, 680.0)));
        assert!(w <= 1280.0 * 0.92 && h <= 680.0 * 0.92);
        assert_eq!(
            default_window_size(Some((5120.0, 2800.0))),
            (1600.0, 1000.0)
        );
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

    const WORK: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    #[test]
    fn a_window_is_centred_in_the_work_area() {
        assert_eq!(centered(WORK, 1000, 800), (460, 120));
        // A second monitor to the left of the first (negative coordinates).
        let left = Rect {
            left: -2560,
            top: 0,
            right: 0,
            bottom: 1400,
        };
        assert_eq!(centered(left, 1280, 800), (-1920, 300));
        // Larger than the area: pinned to its corner, never off the top-left.
        assert_eq!(centered(WORK, 3000, 2000), (0, 0));
    }

    #[test]
    fn a_window_hanging_over_an_edge_is_pulled_back() {
        let win = |l, t, w, h| Rect {
            left: l,
            top: t,
            right: l + w,
            bottom: t + h,
        };
        // Fits: untouched.
        assert_eq!(fitted(WORK, win(100, 100, 1000, 700)), (100, 100));
        // Half behind the taskbar.
        assert_eq!(fitted(WORK, win(100, 700, 1000, 700)), (100, 340));
        // Off the right edge and the left edge.
        assert_eq!(fitted(WORK, win(1500, 100, 1000, 700)), (920, 100));
        assert_eq!(fitted(WORK, win(-600, 100, 1000, 700)), (0, 100));
        // A monitor that is gone: far outside, pulled onto this one.
        assert_eq!(fitted(WORK, win(5000, 3000, 1000, 700)), (920, 340));
        // Larger than the area: pinned to the corner.
        assert_eq!(fitted(WORK, win(50, 50, 3000, 2000)), (0, 0));
    }
}

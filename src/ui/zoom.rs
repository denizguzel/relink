//! Zoom: the whole window's content drawn larger or smaller. Rune's `scale`
//! changes how an element is drawn, not its layout, so the content is laid out
//! in a box of window size divided by the zoom and then scaled back up to fit.

use rune::{El, col, el};

pub const MIN: u32 = 70;
pub const MAX: u32 = 150;
pub const STEP: u32 = 10;

/// The next zoom level after a key press: `up` bigger, otherwise smaller,
/// kept in range and on the step grid.
pub fn step(current: u32, up: bool) -> u32 {
    let now = current.clamp(MIN, MAX);
    if up {
        (now / STEP * STEP + STEP).min(MAX)
    } else {
        (now.div_ceil(STEP) * STEP).saturating_sub(STEP).max(MIN)
    }
}

/// The size to lay the content out at so that, scaled by `percent`, it fills
/// a window of `window` points.
pub fn layout_size(window: (f32, f32), percent: u32) -> (f32, f32) {
    let z = percent.clamp(MIN, MAX) as f32 / 100.0;
    (window.0 / z, window.1 / z)
}

/// Wraps the window's content. At 100 %, or before the window's size is
/// known, the content is returned as it is.
pub fn wrap(content: El, window: Option<(f32, f32)>, percent: u32) -> El {
    let Some(window) = window.filter(|_| percent != 100) else {
        return content;
    };
    let (w, h) = layout_size(window, percent);
    // Absolute, so the parent does not stretch the box back to the window's size.
    col().key("zoom-frame").w_full().h_full().child(
        el().key("zoom")
            .absolute(0.0, 0.0)
            .w(w)
            .h(h)
            .origin(0.0, 0.0)
            .scale(percent.clamp(MIN, MAX) as f32 / 100.0)
            .child(content),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range_and_on_the_grid() {
        assert_eq!(step(100, true), 110);
        assert_eq!(step(100, false), 90);
        assert_eq!(step(MAX, true), MAX);
        assert_eq!(step(MIN, false), MIN);
        assert_eq!(step(105, true), 110);
        assert_eq!(step(105, false), 100);
        assert_eq!(
            step(500, false),
            140,
            "an out of range value is clamped first"
        );
    }

    #[test]
    fn content_is_laid_out_smaller_when_zoomed_in() {
        assert_eq!(layout_size((1000.0, 600.0), 125), (800.0, 480.0));
        assert_eq!(layout_size((1000.0, 600.0), 100), (1000.0, 600.0));
    }
}

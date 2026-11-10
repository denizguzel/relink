//! Making the title bar match the window's appearance (Windows).
//!
//! The system draws the title bar from its own light/dark setting. When the
//! app's theme setting forces the other appearance, the bar would be the wrong
//! colour, so it is painted to match; when the app follows the system the
//! system's own bar is restored.

/// What the title bar should look like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleBar {
    /// Leave it to the system (the app follows the system's appearance).
    System,
    /// Draw it light or dark with this background colour (sRGB bytes).
    Forced { dark: bool, background: [u8; 3] },
}

/// Applies `bar` to this process's window. `system_dark` is the system's own
/// appearance, to go back to when the app follows it. Cheap to call every
/// frame: the system is only asked when something changed.
#[cfg(windows)]
pub fn match_title_bar(bar: TitleBar, system_dark: bool) {
    use std::sync::Mutex;
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;

    // Windows 10 20H1+ (dark mode) and Windows 11 (caption colour).
    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_CAPTION_COLOR: u32 = 35;
    const DWMWA_COLOR_DEFAULT: u32 = 0xFFFF_FFFF;
    static APPLIED: Mutex<Option<(TitleBar, bool)>> = Mutex::new(None);

    let wanted = (bar, system_dark);
    let mut applied = APPLIED.lock().unwrap_or_else(|e| e.into_inner());
    if *applied == Some(wanted) {
        return;
    }
    // The window may not exist yet on the first frames: try again next time.
    let Some(hwnd) = super::own_window::own_window() else {
        return;
    };
    let (dark, color) = match bar {
        TitleBar::System => (system_dark, DWMWA_COLOR_DEFAULT),
        TitleBar::Forced { dark, background } => {
            let [r, g, b] = background;
            (dark, u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16)
        }
    };
    let dark = i32::from(dark);
    // SAFETY: both attributes take a 4-byte value that outlives the call; a
    // system that does not know an attribute just returns an error.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark as *const i32).cast(),
            4,
        );
        DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR, (&color as *const u32).cast(), 4);
    }
    *applied = Some(wanted);
}

#[cfg(not(windows))]
pub fn match_title_bar(_bar: TitleBar, _system_dark: bool) {}

//! The window's current geometry, for remembering where the user put it.

/// This process's visible top-level window: top-left in physical pixels and
/// size in logical pixels. Rune does not report the window's geometry, so
/// Windows is asked directly; elsewhere the window manager keeps it.
#[cfg(windows)]
pub fn window_geometry() -> Option<crate::config::WindowGeom> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect, IsIconic};

    let hwnd = super::own_window::own_window()?;
    // SAFETY: the RECTs are plain data filled by the calls.
    unsafe {
        if IsIconic(hwnd) != 0 {
            return None;
        }
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut r) == 0 {
            return None;
        }
        // The size is the client area (what `AppSpec::size` means), not the frame.
        let mut client: RECT = std::mem::zeroed();
        if GetClientRect(hwnd, &mut client) == 0 {
            return None;
        }
        let scale = (GetDpiForSystem().max(96)) as f32 / 96.0;
        Some(crate::config::WindowGeom {
            x: r.left,
            y: r.top,
            width: client.right as f32 / scale,
            height: client.bottom as f32 / scale,
        })
    }
}

#[cfg(not(windows))]
pub fn window_geometry() -> Option<crate::config::WindowGeom> {
    None
}

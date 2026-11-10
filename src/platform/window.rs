//! Screen and window geometry: the first size, and remembering the position.

/// The usable area of the primary screen in logical pixels (the taskbar
/// excluded), when the system can say. Windows only: elsewhere the window
/// manager places windows itself.
#[cfg(windows)]
pub fn screen_work_area() -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SystemParametersInfoW};
    // SAFETY: SPI_GETWORKAREA fills the RECT it is given.
    let (rect, dpi) = unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut rect as *mut RECT).cast(), 0) == 0 {
            return None;
        }
        (rect, GetDpiForSystem())
    };
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    let (w, h) = (
        (rect.right - rect.left) as f32 / scale,
        (rect.bottom - rect.top) as f32 / scale,
    );
    (w > 0.0 && h > 0.0).then_some((w, h))
}

#[cfg(not(windows))]
pub fn screen_work_area() -> Option<(f32, f32)> {
    None
}

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

/// Moves this process's window to `(x, y)` (physical pixels), unless that
/// would leave it off every screen (a monitor that is gone).
#[cfg(windows)]
pub fn move_window(x: i32, y: i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };

    // SAFETY: plain Win32 calls on this process's own window.
    unsafe {
        let (vx, vy) = (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
        );
        let (vw, vh) = (
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        );
        // The title bar must stay reachable: its top-left well inside the desktop.
        if !super::geometry::reachable(x, y, (vx, vy, vw, vh)) {
            return;
        }
        if let Some(hwnd) = super::own_window::own_window() {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn move_window(_x: i32, _y: i32) {}

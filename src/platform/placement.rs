//! Where the window opens: on the monitor the user is working on, centred, and
//! never hanging over the taskbar or off a monitor that is gone.
//!
//! "The monitor the user is working on" is the one under the mouse pointer: it
//! is where they just clicked the shortcut or the installer's "Launch" button.

/// The usable area (taskbar excluded) of the active monitor in logical pixels,
/// when the system can say, to size the window for it. Windows only: elsewhere
/// the window manager places windows itself.
#[cfg(windows)]
pub fn screen_work_area() -> Option<(f32, f32)> {
    let (work, scale) = imp::work_area_at_cursor()?;
    Some((work.width() as f32 / scale, work.height() as f32 / scale))
}

#[cfg(not(windows))]
pub fn screen_work_area() -> Option<(f32, f32)> {
    None
}

/// Puts this process's window where it belongs. With a `saved` top-left
/// (physical pixels) it goes back there, pulled inside the monitor it lands on
/// when it would hang over an edge or that monitor is gone; without one it is
/// centred on the active monitor.
#[cfg(windows)]
pub fn place_window(saved: Option<(i32, i32)>) {
    imp::place_window(saved);
}

#[cfg(not(windows))]
pub fn place_window(_saved: Option<(i32, i32)>) {}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetWindowRect, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };

    use crate::platform::geometry::{Rect, centered, fitted};
    use crate::platform::own_window::own_window;

    /// The work area of `monitor` in physical pixels, and its display scale.
    fn work_area(monitor: HMONITOR) -> Option<(Rect, f32)> {
        // SAFETY: MONITORINFO is plain data with its size set; the DPI values
        // are written only on success.
        unsafe {
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return None;
            }
            let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
            let scale = if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == 0
                && dpi_x > 0
            {
                dpi_x as f32 / 96.0
            } else {
                1.0
            };
            let w: RECT = info.rcWork;
            let rect = Rect {
                left: w.left,
                top: w.top,
                right: w.right,
                bottom: w.bottom,
            };
            Some((rect, scale))
        }
    }

    /// The work area of the monitor containing `(x, y)`, or the nearest one.
    fn monitor_at(x: i32, y: i32) -> Option<(Rect, f32)> {
        // SAFETY: a plain lookup by point.
        let monitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
        work_area(monitor)
    }

    /// The work area and scale of the monitor under the mouse pointer.
    pub(super) fn work_area_at_cursor() -> Option<(Rect, f32)> {
        let mut at = POINT { x: 0, y: 0 };
        // SAFETY: GetCursorPos fills the POINT it is given.
        if unsafe { GetCursorPos(&mut at) } == 0 {
            return None;
        }
        monitor_at(at.x, at.y)
    }

    pub(super) fn place_window(saved: Option<(i32, i32)>) {
        let Some(hwnd) = own_window() else {
            return;
        };
        // SAFETY: plain Win32 calls on this process's own window.
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut r) == 0 {
                return;
            }
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            let target = match saved {
                Some((x, y)) => {
                    // The monitor the saved window lands on (the nearest one
                    // when its own is gone).
                    let Some((work, _)) = monitor_at(x + w / 2, y + h / 2) else {
                        return;
                    };
                    fitted(
                        work,
                        Rect {
                            left: x,
                            top: y,
                            right: x + w,
                            bottom: y + h,
                        },
                    )
                }
                None => {
                    let Some((work, _)) = work_area_at_cursor() else {
                        return;
                    };
                    centered(work, w, h)
                }
            };
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                target.0,
                target.1,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

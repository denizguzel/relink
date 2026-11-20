//! This process's main window (Windows): Rune does not hand out its handle, so
//! it is found among the top-level windows.

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongW, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, WS_EX_TOOLWINDOW,
};

unsafe extern "system" fn each(hwnd: HWND, found: LPARAM) -> i32 {
    // SAFETY: `found` points at the Option<HWND> in `own_window` for the whole call.
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != GetCurrentProcessId()
            || IsWindowVisible(hwnd) == 0
            || !GetWindow(hwnd, GW_OWNER).is_null()
            || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0
        {
            return 1;
        }
        // The tray and notification helpers keep visible windows of no size;
        // the app window always has one (a minimized window keeps its own).
        let mut r: RECT = std::mem::zeroed();
        let sized = GetWindowRect(hwnd, &mut r) != 0 && r.right > r.left && r.bottom > r.top;
        if sized || IsIconic(hwnd) != 0 {
            *(found as *mut Option<HWND>) = Some(hwnd);
            return 0;
        }
    }
    1
}

/// The visible, unowned top-level window of this process that has a size
/// (or is minimized), if it has one.
pub(super) fn own_window() -> Option<HWND> {
    let mut found: Option<HWND> = None;
    // SAFETY: the callback only writes the out parameter.
    unsafe {
        EnumWindows(Some(each), (&mut found as *mut Option<HWND>) as LPARAM);
    }
    found
}

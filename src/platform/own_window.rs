//! This process's main window (Windows): Rune does not hand out its handle, so
//! it is found among the top-level windows.

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible,
};

unsafe extern "system" fn each(hwnd: HWND, found: LPARAM) -> i32 {
    // SAFETY: `found` points at the Option<HWND> in `own_window` for the whole call.
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId()
            && IsWindowVisible(hwnd) != 0
            && GetWindow(hwnd, GW_OWNER).is_null()
        {
            *(found as *mut Option<HWND>) = Some(hwnd);
            return 0;
        }
    }
    1
}

/// The visible, unowned top-level window of this process, if it has one.
pub(super) fn own_window() -> Option<HWND> {
    let mut found: Option<HWND> = None;
    // SAFETY: the callback only writes the out parameter.
    unsafe {
        EnumWindows(Some(each), (&mut found as *mut Option<HWND>) as LPARAM);
    }
    found
}

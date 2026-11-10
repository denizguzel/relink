//! Waiting for another process to end.

use std::time::Duration;

/// Returns once process `pid` has exited, or after `timeout`. A process that
/// is already gone counts as exited.
#[cfg(windows)]
pub fn wait_for_exit(pid: u32, timeout: Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
    const SYNCHRONIZE: u32 = 0x0010_0000;
    // SAFETY: Plain handle calls; the handle is closed before returning.
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return;
        }
        WaitForSingleObject(handle, timeout.as_millis().min(u32::MAX as u128 - 1) as u32);
        CloseHandle(handle);
    }
}

#[cfg(not(windows))]
pub fn wait_for_exit(pid: u32, timeout: Duration) {
    let proc = std::path::PathBuf::from(format!("/proc/{pid}"));
    let start = std::time::Instant::now();
    while proc.exists() && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exited_process_returns_at_once() {
        let mut child = std::process::Command::new(if cfg!(windows) { "cmd" } else { "true" })
            .args(if cfg!(windows) {
                vec!["/c", "exit"]
            } else {
                vec![]
            })
            .spawn()
            .unwrap();
        let pid = child.id();
        child.wait().unwrap();
        let start = std::time::Instant::now();
        wait_for_exit(pid, Duration::from_secs(10));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}

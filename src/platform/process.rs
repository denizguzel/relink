//! Waiting for another process to end, and the previous version after an update.

use std::time::Duration;

/// Waits until process `pid` has exited, at most `timeout`; returns whether it
/// did. A process that is already gone counts as exited.
#[cfg(windows)]
pub fn wait_for_exit(pid: u32, timeout: Duration) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
    const SYNCHRONIZE: u32 = 0x0010_0000;
    // SAFETY: Plain handle calls; the handle is closed before returning.
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return true;
        }
        let r = WaitForSingleObject(handle, timeout.as_millis().min(u32::MAX as u128 - 1) as u32);
        CloseHandle(handle);
        r != WAIT_TIMEOUT
    }
}

#[cfg(not(windows))]
pub fn wait_for_exit(pid: u32, timeout: Duration) -> bool {
    let proc = std::path::PathBuf::from(format!("/proc/{pid}"));
    let start = std::time::Instant::now();
    while proc.exists() && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(100));
    }
    !proc.exists()
}

/// The names the previous version runs under: the program's own file name
/// (Windows keeps the name it started with) or that name with the `.old` the
/// update renamed it to (Linux follows the rename).
fn previous_names(exe_name: &str) -> Vec<String> {
    vec![exe_name.to_string(), format!("{exe_name}.old")]
}

/// After an update: waits for the previous version (`pid`) to quit. If it is
/// still running after `timeout` (stuck while quitting), it is ended, but only
/// when that process really is Re-Link, so a reused process id is never hit.
/// Otherwise the new version would find it still running and exit too.
pub fn end_previous(pid: u32, timeout: Duration) {
    if wait_for_exit(pid, timeout) {
        return;
    }
    let Some(name) = std::env::current_exe()
        .ok()
        .and_then(|p| Some(p.file_name()?.to_string_lossy().into_owned()))
    else {
        return;
    };
    let ours = crate::net::processes(&previous_names(&name))
        .is_ok_and(|ps| ps.iter().any(|p| p.pid == pid));
    if ours && crate::net::terminate(pid).is_ok() {
        wait_for_exit(pid, Duration::from_secs(5));
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
        assert!(wait_for_exit(pid, Duration::from_secs(10)));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_running_process_times_out() {
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        assert!(!wait_for_exit(child.id(), Duration::from_millis(200)));
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn the_previous_version_runs_under_its_name_or_the_old_one() {
        assert_eq!(
            previous_names("relink.exe"),
            ["relink.exe", "relink.exe.old"]
        );
    }
}

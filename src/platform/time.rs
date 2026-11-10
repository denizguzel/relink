//! Local wall-clock time, for the activity log.

/// The local wall-clock time as HH:MM:SS, for the activity log.
#[cfg(windows)]
pub fn local_time() -> String {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: SYSTEMTIME is plain data filled by the call.
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    format!("{:02}:{:02}:{:02}", t.wHour, t.wMinute, t.wSecond)
}

#[cfg(unix)]
pub fn local_time() -> String {
    // SAFETY: localtime_r fills the zeroed tm from a valid time_t.
    let tm = unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm
    };
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

#[cfg(not(any(windows, unix)))]
pub fn local_time() -> String {
    let s = crate::config::now_unix() % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// A unix time as local (year, month, day, hour, minute, second).
#[cfg(windows)]
pub fn local_parts(ts: u64) -> (u32, u32, u32, u32, u32, u32) {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
    let ticks = (ts + 11_644_473_600) * 10_000_000;
    let ft = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    // SAFETY: plain-data out parameters filled by the calls.
    let t = unsafe {
        let mut utc = std::mem::zeroed();
        let mut local = std::mem::zeroed();
        if FileTimeToSystemTime(&ft, &mut utc) == 0 {
            return (1970, 1, 1, 0, 0, 0);
        }
        if SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) == 0 {
            local = utc;
        }
        local
    };
    (
        t.wYear as u32,
        t.wMonth as u32,
        t.wDay as u32,
        t.wHour as u32,
        t.wMinute as u32,
        t.wSecond as u32,
    )
}

#[cfg(unix)]
pub fn local_parts(ts: u64) -> (u32, u32, u32, u32, u32, u32) {
    // SAFETY: localtime_r fills the zeroed tm from a valid time_t.
    let tm = unsafe {
        let t = ts as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm
    };
    (
        tm.tm_year as u32 + 1900,
        tm.tm_mon as u32 + 1,
        tm.tm_mday as u32,
        tm.tm_hour as u32,
        tm.tm_min as u32,
        tm.tm_sec as u32,
    )
}

#[cfg(not(any(windows, unix)))]
pub fn local_parts(ts: u64) -> (u32, u32, u32, u32, u32, u32) {
    let s = ts % 86_400;
    (
        1970,
        1,
        1,
        (s / 3600) as u32,
        (s / 60 % 60) as u32,
        (s % 60) as u32,
    )
}

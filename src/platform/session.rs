//! Whether the user's session is locked (Win+L, the lock screen after sleep,
//! or a Linux desktop's screen lock). Polled by the monitor; no window or
//! message loop needed.

/// `Some(true)` while the session is locked, `Some(false)` while it is not,
/// `None` when the system does not say.
#[cfg(windows)]
pub fn session_locked() -> Option<bool> {
    use windows_sys::Win32::System::RemoteDesktop::{
        WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK,
        WTS_SESSIONSTATE_UNLOCK, WTSFreeMemory, WTSINFOEXW, WTSQuerySessionInformationW,
        WTSSessionInfoEx,
    };
    let mut buf: windows_sys::core::PWSTR = std::ptr::null_mut();
    let mut len = 0u32;
    // SAFETY: on success the call hands back a WTSINFOEXW the system
    // allocated, which is read once and freed with WTSFreeMemory.
    unsafe {
        if WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buf,
            &mut len,
        ) == 0
            || buf.is_null()
        {
            return None;
        }
        let info = &*(buf as *const WTSINFOEXW);
        let flags = if info.Level == 1 && len as usize >= size_of::<WTSINFOEXW>() {
            Some(info.Data.WTSInfoExLevel1.SessionFlags as u32)
        } else {
            None
        };
        WTSFreeMemory(buf.cast());
        match flags? {
            WTS_SESSIONSTATE_LOCK => Some(true),
            WTS_SESSIONSTATE_UNLOCK => Some(false),
            _ => None,
        }
    }
}

/// Linux: logind's `LockedHint`, which GNOME, KDE and other desktops set
/// while their screen lock shows. Desktops that never set it read as unlocked.
#[cfg(target_os = "linux")]
pub fn session_locked() -> Option<bool> {
    let session = std::env::var("XDG_SESSION_ID")
        .ok()
        .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "self".into());
    let out = std::process::Command::new("loginctl")
        .args(["show-session", &session, "--property=LockedHint", "--value"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_locked_hint(&String::from_utf8_lossy(&out.stdout))
}

/// `loginctl … --value` prints "yes" or "no".
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_locked_hint(text: &str) -> Option<bool> {
    match text.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn session_locked() -> Option<bool> {
    None
}

#[cfg(test)]
mod tests {
    use super::parse_locked_hint;

    #[test]
    fn logind_answers_yes_or_no() {
        assert_eq!(parse_locked_hint("yes\n"), Some(true));
        assert_eq!(parse_locked_hint("no\n"), Some(false));
        assert_eq!(parse_locked_hint(""), None);
    }
}

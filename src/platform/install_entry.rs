//! The "Installed apps" entry the Windows installer writes (per user, under
//! HKCU). The in-app updater swaps only the executable, so the entry would keep
//! showing the installed version; Re-Link brings it up to date at start, and
//! only when it runs from the installed folder (a portable copy leaves it be).

use std::path::Path;

/// The installer's AppId, from `windows-app-id` in Cargo.toml (via build.rs).
pub const WINDOWS_APP_ID: &str = env!("RELINK_WINDOWS_APP_ID");

/// The uninstall key Inno Setup writes for that AppId.
#[cfg_attr(not(windows), allow(dead_code))]
fn entry_key() -> String {
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{{{WINDOWS_APP_ID}}}_is1")
}

/// Whether `install_location` (as the installer wrote it) is `exe`'s folder.
/// Windows paths compare without case and without a trailing separator.
#[cfg_attr(not(windows), allow(dead_code))]
fn runs_from(install_location: &str, exe: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches(['\\', '/']).to_lowercase();
    // Split by hand: the paths are Windows paths, also when tested on Linux.
    let Some((dir, _)) = exe.to_str().and_then(|e| e.rsplit_once(['\\', '/'])) else {
        return false;
    };
    !install_location.trim().is_empty() && norm(install_location) == norm(dir)
}

/// "0.5.0" → (0, 5), for the entry's numeric version values.
#[cfg_attr(not(windows), allow(dead_code))]
fn major_minor(version: &str) -> Option<(u32, u32)> {
    let mut parts = version.split(['.', '-']);
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

/// Updates the entry's version when this executable is the installed one.
/// Returns the version it replaced, if it changed anything.
#[cfg(windows)]
pub fn sync_install_entry(exe: &Path, version: &str) -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(entry_key(), KEY_READ | KEY_SET_VALUE)
        .ok()?;
    let location: String = key.get_value("InstallLocation").ok()?;
    let shown: String = key.get_value("DisplayVersion").unwrap_or_default();
    if !runs_from(&location, exe) || shown == version {
        return None;
    }
    key.set_value("DisplayVersion", &version).ok()?;
    if let Some((major, minor)) = major_minor(version) {
        for (name, value) in [
            ("MajorVersion", major),
            ("MinorVersion", minor),
            ("VersionMajor", major),
            ("VersionMinor", minor),
        ] {
            let _ = key.set_value(name, &value);
        }
    }
    Some(shown)
}

#[cfg(not(windows))]
pub fn sync_install_entry(_exe: &Path, _version: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_installed_folder_counts() {
        let exe = Path::new(r"C:\Users\x\AppData\Local\Programs\Re-Link\relink.exe");
        assert!(runs_from(
            r"C:\Users\x\AppData\Local\Programs\Re-Link\",
            exe
        ));
        assert!(runs_from(r"c:\users\X\appdata\local\programs\re-link", exe));
        assert!(!runs_from(r"C:\Users\x\Desktop\Re-Link-portable\", exe));
        assert!(!runs_from("", exe));
    }

    #[test]
    fn the_entry_key_is_inno_setups_name_for_the_app_id() {
        assert_eq!(WINDOWS_APP_ID.len(), 36);
        assert!(entry_key().ends_with(&format!("\\{{{WINDOWS_APP_ID}}}_is1")));
    }

    #[test]
    fn versions_split_into_major_and_minor() {
        assert_eq!(major_minor("0.5.0"), Some((0, 5)));
        assert_eq!(major_minor("1.12.3-rc.1"), Some((1, 12)));
        assert_eq!(major_minor("garbage"), None);
    }
}

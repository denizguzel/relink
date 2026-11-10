//! The VPN client: opening FortiClient on its Remote Access page so the user can
//! connect. Re-Link never connects by itself: the company VPN signs in through
//! the browser (SSO), and FortiClient offers no supported way to start an SSL
//! VPN tunnel from another program (its COM API only knows IPsec tunnels).

use std::path::{Path, PathBuf};

/// FortiClient's own argument for "the network is locked down": it opens the
/// main window on the Remote Access page and does nothing else. Undocumented,
/// so an update may drop it; FortiClient then just opens its main window.
#[cfg_attr(not(windows), allow(dead_code))]
pub const REMOTE_ACCESS_ARG: &str = "--network-locked-down";

#[cfg_attr(not(windows), allow(dead_code))]
const DEFAULT_DIR: &str = r"C:\Program Files\Fortinet\FortiClient";

/// The FortiClient executable in `install_dir` (the registry's INSTALLDIR),
/// or in the default folder when the registry does not say.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn client_exe_in(install_dir: Option<&str>) -> PathBuf {
    let dir = install_dir
        .map(|d| d.trim().trim_end_matches(['\\', '/']))
        .filter(|d| !d.is_empty())
        .unwrap_or(DEFAULT_DIR);
    Path::new(dir).join("FortiClient.exe")
}

#[cfg(windows)]
fn client_exe() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let dir = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Fortinet\FortiClient")
        .and_then(|k| k.get_value::<String, _>("INSTALLDIR"))
        .ok();
    Some(client_exe_in(dir.as_deref())).filter(|p| p.is_file())
}

/// Whether a VPN client Re-Link can open is installed.
#[cfg(windows)]
pub fn vpn_client_available() -> bool {
    client_exe().is_some()
}

/// Opens the VPN client on its connection page. Returns whether it started.
#[cfg(windows)]
pub fn open_vpn_client() -> bool {
    let Some(exe) = client_exe() else {
        return false;
    };
    std::process::Command::new(&exe)
        .arg(REMOTE_ACCESS_ARG)
        .spawn()
        .or_else(|_| std::process::Command::new(&exe).spawn())
        .is_ok()
}

#[cfg(not(windows))]
pub fn vpn_client_available() -> bool {
    false
}

#[cfg(not(windows))]
pub fn open_vpn_client() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_folder_wins_and_trailing_separators_are_dropped() {
        let p = client_exe_in(Some(r"D:\Apps\FortiClient\"));
        assert!(p.to_string_lossy().starts_with(r"D:\Apps\FortiClient"));
        assert!(p.to_string_lossy().ends_with("FortiClient.exe"));
        assert!(!p.to_string_lossy().contains(r"\\"));
    }

    #[test]
    fn without_the_registry_the_default_folder_is_used() {
        for dir in [None, Some(""), Some("  ")] {
            assert!(client_exe_in(dir).starts_with(DEFAULT_DIR), "{dir:?}");
        }
    }
}

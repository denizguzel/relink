//! Starting with the user's session: HKCU Run on Windows, an XDG autostart
//! entry elsewhere. No administrator rights either way.

use std::path::Path;

#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const RUN_VALUE: &str = "Re-Link";
/// The argument autostart passes, so the app starts in the tray.
pub const HIDDEN_ARG: &str = "--hidden";

/// Whether `exe` is registered to start with Windows.
#[cfg(windows)]
pub fn startup_enabled(exe: &Path) -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok_and(|v| {
            v.to_lowercase()
                .contains(&exe.display().to_string().to_lowercase())
        })
}

/// Registers or removes `exe` under HKCU Run (no admin rights needed).
#[cfg(windows)]
pub fn set_startup(exe: &Path, on: bool) -> std::io::Result<()> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
    if on {
        key.set_value(RUN_VALUE, &format!("\"{}\" {HIDDEN_ARG}", exe.display()))
    } else {
        match key.delete_value(RUN_VALUE) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    }
}

#[cfg(not(windows))]
fn autostart_file() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("autostart").join("relink.desktop"))
}

/// Whether `exe` has an XDG autostart entry.
#[cfg(not(windows))]
pub fn startup_enabled(exe: &Path) -> bool {
    autostart_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .is_some_and(|t| t.contains(&exe.display().to_string()))
}

/// Writes or removes `~/.config/autostart/relink.desktop`.
#[cfg(not(windows))]
pub fn set_startup(exe: &Path, on: bool) -> std::io::Result<()> {
    let f = autostart_file().ok_or_else(|| std::io::Error::other("no HOME"))?;
    if !on {
        return match std::fs::remove_file(&f) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        };
    }
    std::fs::create_dir_all(f.parent().unwrap())?;
    std::fs::write(
        &f,
        format!(
            "[Desktop Entry]\nType=Application\nName=Re-Link\nExec=\"{}\" {HIDDEN_ARG}\nX-GNOME-Autostart-enabled=true\nTerminal=false\n",
            exe.display()
        ),
    )
}

//! The operating system's own `curl` (Windows 10+ and Ubuntu ship it): HTTPS
//! without a TLS library of our own, following the machine's certificate
//! settings.

use std::process::Command;

/// A `curl` command that opens no console window.
#[cfg(windows)]
pub fn curl() -> Command {
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // The full path: a bare name is looked up in the program's own folder
    // first, where a planted curl.exe would run instead.
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let mut cmd = Command::new(Path::new(&root).join("System32").join("curl.exe"));
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
pub fn curl() -> Command {
    Command::new("curl")
}

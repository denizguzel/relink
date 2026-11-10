//! Asking GitHub for the latest release, with the operating system's own
//! `curl` (Windows 10+ and Ubuntu ship it). That adds no TLS library and
//! follows the machine's proxy and certificate settings.

use std::path::Path;
use std::process::{Command, Stdio};

use super::REPO;

/// The JSON of the repository's latest published release.
pub(super) fn latest_json() -> Result<String, String> {
    run(
        &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        None,
        20,
        1_000_000,
    )
}

/// A small text file (the checksums).
pub(super) fn get_text(url: &str) -> Result<String, String> {
    run(url, None, 30, 1_000_000)
}

/// A release file, saved to `dest`.
pub(super) fn download(url: &str, dest: &Path) -> Result<(), String> {
    run(url, Some(dest), 900, 200_000_000).map(|_| ())
}

fn run(url: &str, dest: Option<&Path>, seconds: u32, max_bytes: u64) -> Result<String, String> {
    let mut cmd = curl();
    cmd.args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--proto", "=https", "--proto-redir", "=https"])
        .args(["--max-time", &seconds.to_string()])
        .args(["--max-filesize", &max_bytes.to_string()])
        .args(["--header", "Accept: application/vnd.github+json"])
        .args(["--header", "User-Agent: relink-update"]);
    if let Some(dest) = dest {
        cmd.arg("--output").arg(dest);
    }
    let out = cmd
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl: {}", err.trim()));
    }
    String::from_utf8(out.stdout).or_else(|_| Ok(String::new()))
}

#[cfg(windows)]
fn curl() -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // The full path: a bare name is looked up in the program's own folder
    // first, where a planted curl.exe would run instead.
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let mut cmd = Command::new(Path::new(&root).join("System32").join("curl.exe"));
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn curl() -> Command {
    Command::new("curl")
}

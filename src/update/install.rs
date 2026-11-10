//! Installing a release in place: download the platform's executable next to
//! the running one, check it against the release's checksums, swap it in and
//! start it. The running file is renamed, never overwritten, so it works on
//! Windows too; the old copy is deleted by the new one at its start.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::asset::{SUMS, parse_sums, sha256_file};
use super::fetch;
use super::release::Download;

/// What the new process is told so it waits for this one to exit.
pub const WAIT_ARG: &str = "--wait-pid";

/// `relink.exe` → `relink.exe.old`.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// Downloads, verifies and swaps in the update, then starts it. The caller
/// quits right after, and the new process waits for that.
pub fn install(download: &Download) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("own path: {e}"))?;
    let new = with_suffix(&exe, ".new");
    let result = fetch_verified(download, &new)
        .and_then(|()| swap(&exe, &new).map_err(|e| format!("replacing the program: {e}")));
    if result.is_err() {
        let _ = fs::remove_file(&new);
    }
    result?;
    Command::new(&exe)
        .arg(WAIT_ARG)
        .arg(std::process::id().to_string())
        .spawn()
        .map_err(|e| format!("starting the new version: {e}"))?;
    Ok(())
}

/// Downloads the file to `dest` and checks it against the release's SHA256SUMS.
fn fetch_verified(download: &Download, dest: &Path) -> Result<(), String> {
    let sums = parse_sums(&fetch::get_text(&download.sums_url)?);
    let want = sums
        .get(&download.name)
        .ok_or_else(|| format!("{SUMS} has no entry for {}", download.name))?;
    fetch::download(&download.asset_url, dest)?;
    let got = sha256_file(dest).map_err(|e| format!("reading the download: {e}"))?;
    if &got != want {
        return Err("the download does not match its checksum".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dest, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("making it executable: {e}"))?;
    }
    Ok(())
}

/// Puts `new` where `exe` is, keeping the old file as `exe.old`. If the
/// second step fails the first is undone, so there is always a program.
fn swap(exe: &Path, new: &Path) -> io::Result<()> {
    let old = with_suffix(exe, ".old");
    let _ = fs::remove_file(&old);
    fs::rename(exe, &old)?;
    if let Err(e) = fs::rename(new, exe) {
        let _ = fs::rename(&old, exe);
        return Err(e);
    }
    Ok(())
}

/// Deletes the previous version left behind by an update.
pub fn remove_old() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = fs::remove_file(with_suffix(&exe, ".old"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("relink-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn swapping_replaces_the_program_and_keeps_the_old_one() {
        let dir = temp("swap");
        let (exe, new) = (dir.join("relink.exe"), dir.join("relink.exe.new"));
        fs::write(&exe, "old").unwrap();
        fs::write(&new, "new").unwrap();
        swap(&exe, &new).unwrap();
        assert_eq!(fs::read_to_string(&exe).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(dir.join("relink.exe.old")).unwrap(),
            "old"
        );
        assert!(!new.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_swap_leaves_the_old_program_in_place() {
        let dir = temp("swapfail");
        let exe = dir.join("relink.exe");
        fs::write(&exe, "old").unwrap();
        let missing = dir.join("relink.exe.new");
        assert!(swap(&exe, &missing).is_err());
        assert_eq!(fs::read_to_string(&exe).unwrap(), "old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_second_update_replaces_the_previous_old_copy() {
        let dir = temp("swap2");
        let (exe, new) = (dir.join("relink"), dir.join("relink.new"));
        fs::write(dir.join("relink.old"), "ancient").unwrap();
        fs::write(&exe, "v1").unwrap();
        fs::write(&new, "v2").unwrap();
        swap(&exe, &new).unwrap();
        assert_eq!(fs::read_to_string(dir.join("relink.old")).unwrap(), "v1");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn suffixes_keep_the_extension_in_the_name() {
        assert_eq!(
            with_suffix(Path::new("/a/relink.exe"), ".new"),
            Path::new("/a/relink.exe.new")
        );
    }
}

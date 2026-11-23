//! Restarts a browser whose sockets live in its main process (Firefox).
//!
//! Re-Link cannot close one socket without administrator rights, so the only
//! way to drop a stuck connection is to end the main process and start the
//! browser again with the command line it had (same profile, same flags).
//! The browser restores its tabs from its own session backup.

use std::time::{Duration, Instant};

use super::{command_line, processes, terminate};

/// Splits a command line into the executable and the rest, the way Windows
/// does for the first token: quoted (`"C:\A B\firefox.exe" -P x`) or up to the
/// first whitespace. The rest is returned as written (no re-quoting).
pub fn split_command_line(cl: &str) -> Option<(String, String)> {
    let cl = cl.trim();
    if let Some(rest) = cl.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some((rest[..end].to_string(), rest[end + 1..].trim().to_string()));
    }
    let (exe, rest) = cl.split_once(char::is_whitespace).unwrap_or((cl, ""));
    (!exe.is_empty()).then(|| (exe.to_string(), rest.trim().to_string()))
}

/// The browser's main process is the one that is not a `-contentproc` child.
pub fn is_main_process(command_line: &str) -> bool {
    !command_line.contains("-contentproc")
}

/// Waits until none of the browser's processes is left, at most `limit`.
fn wait_gone(names: &[String], limit: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if processes(names).is_ok_and(|p| p.is_empty()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

/// Ends the browser (`names`: its executable names) and starts it again.
/// Returns the executable that was started, or why it could not be.
pub fn restart_browser(names: &[String]) -> Result<String, String> {
    let procs = processes(names)?;
    let main = procs
        .iter()
        .filter_map(|p| command_line(p.pid).map(|cl| (p.pid, cl)))
        .find(|(_, cl)| is_main_process(cl))
        .ok_or("the browser is not running")?;
    let (exe, args) =
        split_command_line(&main.1).ok_or("cannot read the browser's command line")?;
    terminate(main.0)?;
    // The content processes quit with their parent; the profile is locked
    // until they are gone, and a browser started too early would refuse.
    for p in procs.iter().filter(|p| p.pid != main.0) {
        let _ = terminate(p.pid);
    }
    if !wait_gone(names, Duration::from_secs(8)) {
        return Err("the browser did not close in time".into());
    }
    spawn(&exe, &args)?;
    Ok(exe)
}

#[cfg(windows)]
fn spawn(exe: &str, args: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // Detached: the browser must outlive Re-Link and not share its console.
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    std::process::Command::new(exe)
        .raw_arg(args)
        .creation_flags(DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("start {exe}: {e}"))
}

#[cfg(not(windows))]
fn spawn(exe: &str, args: &str) -> Result<(), String> {
    std::process::Command::new(exe)
        .args(args.split_whitespace())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("start {exe}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_quoted_executable() {
        assert_eq!(
            split_command_line(r#""C:\Program Files\Mozilla Firefox\firefox.exe" -P work"#),
            Some((
                r"C:\Program Files\Mozilla Firefox\firefox.exe".into(),
                "-P work".into()
            ))
        );
    }

    #[test]
    fn splits_an_unquoted_executable_and_one_without_arguments() {
        assert_eq!(
            split_command_line("firefox -no-remote"),
            Some(("firefox".into(), "-no-remote".into()))
        );
        assert_eq!(
            split_command_line(r#""C:\f.exe""#),
            Some((r"C:\f.exe".into(), String::new()))
        );
        assert_eq!(split_command_line("  "), None);
        assert_eq!(split_command_line(r#""unterminated"#), None);
    }

    #[test]
    fn content_processes_are_not_the_main_process() {
        assert!(is_main_process(r#""C:\Firefox\firefox.exe""#));
        assert!(!is_main_process(
            r#""C:\Firefox\firefox.exe" -contentproc --channel="1" - 42 7 tab"#
        ));
    }
}

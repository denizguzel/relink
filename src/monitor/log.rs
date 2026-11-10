//! The log file: diagnostics, and the activity events kept across restarts.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config;
use crate::ui::Level;

/// Appends timestamped lines to `relink.log`, rotating it past 1 MB.
#[derive(Clone)]
pub struct Log(PathBuf);

impl Log {
    pub fn open(path: PathBuf) -> Log {
        if fs::metadata(&path).is_ok_and(|m| m.len() > 1 << 20) {
            let _ = fs::rename(&path, path.with_extension("log.1"));
        }
        Log(path)
    }
    /// A diagnostic line (not shown in the window's history).
    pub fn line(&self, msg: &str) {
        self.write(msg);
    }
    /// A line that is also an activity event: it is tagged with its level so
    /// the next start can load it back into the window.
    pub fn event(&self, msg: &str, level: Level) {
        self.write(&format!("[{}] {msg}", level_tag(level)));
    }
    fn write(&self, msg: &str) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.0) {
            let _ = writeln!(
                f,
                "{} {} {}",
                config::now_unix(),
                crate::platform::local_time(),
                msg.replace('\n', " ")
            );
        }
    }
}

fn level_tag(level: Level) -> &'static str {
    match level {
        Level::Info => "info",
        Level::Good => "good",
        Level::Warn => "warn",
    }
}

/// One tagged log line (`<unix> <HH:MM:SS> [level] text`) as an event.
pub fn parse_event(line: &str) -> Option<(u64, String, Level)> {
    let mut parts = line.splitn(3, ' ');
    let ts = parts.next()?.parse().ok()?;
    parts.next()?;
    let rest = parts.next()?;
    for level in [Level::Info, Level::Good, Level::Warn] {
        if let Some(text) = rest.strip_prefix(&format!("[{}] ", level_tag(level))) {
            return Some((ts, text.to_string(), level));
        }
    }
    None
}

/// The activity events kept in the log (and its rotated predecessor), newest
/// first, at most `limit`.
pub fn load_history(path: &Path, limit: usize) -> Vec<(u64, String, Level)> {
    let mut out = Vec::new();
    for p in [path.with_extension("log.1"), path.to_path_buf()] {
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        out.extend(text.lines().filter_map(parse_event));
    }
    out.reverse();
    out.truncate(limit);
    out
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn parses_only_tagged_lines() {
        assert_eq!(
            parse_event("1762800000 19:59:37 [warn] Status: stuck"),
            Some((1762800000, "Status: stuck".into(), Level::Warn))
        );
        assert_eq!(parse_event("1762800000 19:59:37 kill-socket failed"), None);
        assert_eq!(parse_event("garbage"), None);
    }

    #[test]
    fn history_is_newest_first_across_rotation() {
        let dir = std::env::temp_dir().join(format!("relink-hist-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let log = dir.join("relink.log");
        fs::write(dir.join("relink.log.1"), "1 00:00:01 [info] old\n").unwrap();
        fs::write(&log, "2 00:00:02 plain\n3 00:00:03 [good] new\n").unwrap();
        let h = load_history(&log, 10);
        assert_eq!(
            h.iter().map(|e| e.1.as_str()).collect::<Vec<_>>(),
            ["new", "old"]
        );
        assert_eq!(load_history(&log, 1).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}

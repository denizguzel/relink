//! `relink.json` (user settings) and `relink-state.json` (what Re-Link
//! learned at runtime), and where both live.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const WINDOW_FILE_NAME: &str = "relink-window.json";
pub const FILE_NAME: &str = "relink.json";
pub const STATE_FILE_NAME: &str = "relink-state.json";
pub const LOG_FILE_NAME: &str = "relink.log";

/// User-editable settings. Missing fields take their defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// "auto", "tr" or "en".
    pub language: String,
    /// "system" (follow the operating system), "light" or "dark".
    pub theme: String,
    /// Restart the browser network service without asking.
    pub auto_fix: bool,
    /// Which notifications to show: "all", "problems" (stuck connections and
    /// missing routes only) or "off".
    pub notifications: String,
    /// The welcome card on the overview has been dismissed.
    pub welcomed: bool,
    /// Closing the window keeps Re-Link running in the tray (otherwise it quits).
    pub close_to_tray: bool,
    /// A start at login (`--hidden`) stays in the tray instead of opening the window.
    pub start_hidden: bool,
    /// How large the window's content is drawn, in percent (see `ui::zoom`).
    pub zoom_percent: u32,
    /// The one-time "still running in the tray" notice has been shown.
    pub tray_notice_shown: bool,
    /// Ask GitHub now and then whether a newer release exists.
    pub check_updates: bool,
    /// How often the network state is checked.
    pub interval_seconds: u64,
    /// Executable names whose connections are inspected.
    pub browsers: Vec<String>,
    /// Always watched; must route through the VPN while it is connected.
    /// Usually empty: bookmarks supply the hosts.
    pub hosts: Vec<String>,
    /// Watch the company hosts bookmarked in the browsers (all profiles).
    pub use_bookmarks: bool,
    /// Company domain suffixes taken from bookmarks.
    pub bookmark_domains: Vec<String>,
    /// Identify the VPN adapter by description.
    pub vpn_adapter_keywords: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            theme: "system".into(),
            auto_fix: false,
            notifications: "problems".into(),
            welcomed: false,
            close_to_tray: true,
            start_hidden: true,
            zoom_percent: 100,
            tray_notice_shown: false,
            check_updates: true,
            interval_seconds: 15,
            browsers: default_browsers(),
            hosts: vec![],
            use_bookmarks: true,
            bookmark_domains: vec![
                "etycloudbss.com".into(),
                "etiyacloud.com".into(),
                "etiya.com".into(),
            ],
            vpn_adapter_keywords: default_vpn_keywords(),
        }
    }
}

/// Browsers Re-Link watches. Chromium-based ones (Chrome with all channels
/// sharing one executable name, Edge, Brave, Chromium) also get their network
/// service restarted. Firefox (all editions) is watched, but has no separate
/// network process, so on Windows it can only be reported.
pub fn default_browsers() -> Vec<String> {
    let names: &[&str] = if cfg!(windows) {
        &[
            "chrome.exe",
            "msedge.exe",
            "brave.exe",
            "chromium.exe",
            "firefox.exe",
        ]
    } else {
        // Firefox keeps its sockets in the main process: only the Linux root
        // helper can close them, so it is watched on Linux alone.
        &[
            "chrome",
            "msedge",
            "brave",
            "chromium",
            "chromium-browser",
            "firefox",
            "firefox-bin",
        ]
    };
    names.iter().map(|s| s.to_string()).collect()
}

/// Windows matches adapter descriptions; Linux has none, so it matches
/// interface names (FortiClient's tunnel, or ppp for openfortivpn).
fn default_vpn_keywords() -> Vec<String> {
    let k: &[&str] = if cfg!(windows) {
        &["Fortinet SSL VPN", "FortiClient"]
    } else {
        &["vpn", "fct", "forti", "ppp"]
    };
    k.iter().map(|s| s.to_string()).collect()
}

impl Config {
    /// The check interval, clamped to 5..=300 seconds.
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_seconds.clamp(5, 300))
    }

    /// Reads `path`. A missing file gives the defaults; a broken one gives
    /// the defaults and the error.
    pub fn load(path: &Path) -> (Config, Option<String>) {
        match fs::read_to_string(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Config::default(), None),
            Err(e) => (Config::default(), Some(e.to_string())),
            Ok(text) => match serde_json::from_str::<Config>(&text) {
                Ok(mut c) => {
                    let d = Config::default();
                    if c.language.is_empty() {
                        c.language = d.language;
                    }
                    if c.interval_seconds == 0 {
                        c.interval_seconds = d.interval_seconds;
                    }
                    if c.browsers.is_empty() {
                        c.browsers = d.browsers;
                    }
                    if c.vpn_adapter_keywords.is_empty() {
                        c.vpn_adapter_keywords = d.vpn_adapter_keywords;
                    }
                    (c, None)
                }
                Err(e) => (Config::default(), Some(e.to_string())),
            },
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        save_json(path, self)
    }
}

/// Writes `value` as indented JSON, atomically.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut data = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    data.push(b'\n');
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

/// Where the window was last: top-left in physical pixels, size in logical
/// pixels. Kept in its own file (`relink-window.json`) because the window and
/// the monitor would otherwise overwrite each other's settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowGeom {
    pub x: i32,
    pub y: i32,
    pub width: f32,
    pub height: f32,
}

impl WindowGeom {
    pub fn load(path: &Path) -> Option<WindowGeom> {
        serde_json::from_slice(&fs::read(path).ok()?).ok()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        save_json(path, self)
    }
}

/// How long a host stays a "known VPN host" after it was last seen routed
/// through the VPN, so IT can drop a host from the split tunnel without
/// Re-Link warning forever.
const LEARN_TTL: u64 = 7 * 24 * 3600;

/// Runtime knowledge persisted between runs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    /// Host -> last time (Unix seconds) one of its addresses routed through
    /// the VPN.
    pub vpn_hosts: BTreeMap<String, u64>,
}

impl State {
    /// Reads `path`; a missing or broken file gives an empty state.
    pub fn load(path: &Path) -> State {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        save_json(path, self)
    }

    /// Records `hosts` as seen on the VPN at `now` and drops expired entries.
    /// Returns whether anything changed that is worth saving (a new host, an
    /// expiry, or a timestamp older than an hour refreshed).
    pub fn learn(&mut self, hosts: &[String], now: u64) -> bool {
        let mut changed = false;
        for h in hosts {
            let prev = self.vpn_hosts.insert(h.clone(), now);
            if prev.is_none_or(|t| now.saturating_sub(t) > 3600) {
                changed = true;
            }
        }
        let before = self.vpn_hosts.len();
        self.vpn_hosts
            .retain(|_, t| now.saturating_sub(*t) <= LEARN_TTL);
        changed || self.vpn_hosts.len() != before
    }

    /// Learned hosts plus the configured ones.
    pub fn required(&self, configured: &[String]) -> std::collections::HashSet<String> {
        self.vpn_hosts
            .keys()
            .cloned()
            .chain(configured.iter().cloned())
            .collect()
    }
}

/// Current Unix time in seconds.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The sorted, de-duplicated union of host lists, without empty entries.
pub fn merge(lists: &[&[String]]) -> Vec<String> {
    let mut out: Vec<String> = lists
        .iter()
        .flat_map(|l| l.iter())
        .filter(|h| !h.is_empty())
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Where settings, state and the log live.
#[derive(Clone, Debug)]
pub struct Paths {
    pub dir: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub log: PathBuf,
    pub window: PathBuf,
    pub exe: PathBuf,
}

impl Paths {
    /// The paths for the running executable, with the per-user data folder
    /// (`%APPDATA%\Re-Link`, or `$XDG_CONFIG_HOME/relink`) as the fallback.
    pub fn detect() -> Paths {
        let exe = std::env::current_exe().unwrap_or_default();
        let fallback = if cfg!(windows) {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join("Re-Link")
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
                .unwrap_or_default()
                .join("relink")
        };
        Paths::resolve(exe, fallback)
    }

    /// The per-user data folder (`fallback`: `%APPDATA%\Re-Link`, or
    /// `~/.config/relink`). Portable use is opt-in: only when a settings file
    /// already sits next to the executable.
    pub fn resolve(exe: PathBuf, fallback: PathBuf) -> Paths {
        let exe_dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        let dir = if exe_dir.join(FILE_NAME).exists() {
            exe_dir
        } else {
            fallback
        };
        Paths {
            config: dir.join(FILE_NAME),
            state: dir.join(STATE_FILE_NAME),
            log: dir.join(LOG_FILE_NAME),
            window: dir.join(WINDOW_FILE_NAME),
            dir,
            exe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("relink-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (c, err) = Config::load(&tmp("missing").join(FILE_NAME));
        assert!(err.is_none());
        assert_eq!(c, Config::default());
    }

    #[test]
    fn partial_file_fills_gaps_and_round_trips() {
        let p = tmp("partial").join(FILE_NAME);
        fs::write(
            &p,
            r#"{"language":"en","autoFix":true,"hosts":[],"browsers":[]}"#,
        )
        .unwrap();
        let (mut c, err) = Config::load(&p);
        assert!(err.is_none());
        assert!(c.auto_fix && c.language == "en" && c.interval_seconds == 15 && c.use_bookmarks);
        assert_eq!(c.browsers, Config::default().browsers);
        c.language = "tr".into();
        c.save(&p).unwrap();
        assert_eq!(Config::load(&p).0, c);
    }

    #[test]
    fn broken_file_reports_error() {
        let p = tmp("broken").join(FILE_NAME);
        fs::write(&p, "{").unwrap();
        let (c, err) = Config::load(&p);
        assert!(err.is_some());
        assert_eq!(c, Config::default());
    }

    #[test]
    fn interval_is_clamped() {
        let c = |s| Config {
            interval_seconds: s,
            ..Config::default()
        };
        assert_eq!(c(1).interval(), Duration::from_secs(5));
        assert_eq!(c(9999).interval(), Duration::from_secs(300));
    }

    #[test]
    fn state_learns_expires_and_round_trips() {
        let p = tmp("state").join(STATE_FILE_NAME);
        let mut s = State::load(&p);
        let now = 1_800_000_000;
        let old = now - 8 * 24 * 3600;
        assert!(s.learn(&["a".into(), "b".into()], old));
        assert!(
            !s.learn(&["a".into()], old + 60),
            "a refresh within an hour is not a change"
        );
        assert!(s.learn(&["a".into()], now), "b expires");
        assert_eq!(s.vpn_hosts.keys().collect::<Vec<_>>(), vec!["a"]);
        s.save(&p).unwrap();
        let req = State::load(&p).required(&["c".into()]);
        assert!(req.contains("a") && req.contains("c") && !req.contains("b"));
    }

    #[test]
    fn merge_sorts_and_dedups() {
        let a = vec!["b".to_string(), "a".to_string()];
        let b = vec!["a".to_string(), String::new(), "c".to_string()];
        assert_eq!(merge(&[&a, &b]), vec!["a", "b", "c"]);
    }

    #[test]
    fn paths_use_user_dir_unless_portable_marker() {
        let exe_dir = tmp("exe");
        let fb = tmp("fallback");
        assert_eq!(
            Paths::resolve(exe_dir.join("relink.exe"), fb.clone()).dir,
            fb
        );
        fs::write(exe_dir.join(FILE_NAME), "{}").unwrap();
        assert_eq!(Paths::resolve(exe_dir.join("relink.exe"), fb).dir, exe_dir);
    }

    #[test]
    fn new_settings_have_quiet_defaults() {
        let c = Config::default();
        assert_eq!(c.notifications, "problems");
        assert!(!c.welcomed && !c.tray_notice_shown);
        assert_eq!(c.theme, "system");
    }

    #[test]
    fn an_old_settings_file_gets_the_new_fields() {
        let p = tmp("old").join(FILE_NAME);
        fs::write(&p, r#"{"language":"tr","intervalSeconds":10}"#).unwrap();
        let (c, err) = Config::load(&p);
        assert!(err.is_none());
        assert_eq!(c.interval_seconds, 10);
        assert_eq!(c.notifications, "problems");
        assert!(!c.welcomed);
    }

    #[test]
    fn the_new_flags_are_saved_in_camel_case() {
        let p = tmp("flags").join(FILE_NAME);
        let c = Config {
            welcomed: true,
            tray_notice_shown: true,
            ..Config::default()
        };
        c.save(&p).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("\"welcomed\": true") && text.contains("\"trayNoticeShown\": true"));
        assert_eq!(Config::load(&p).0, c);
    }

    #[test]
    fn window_geometry_round_trips_and_a_missing_file_is_none() {
        let p = tmp("geom").join(WINDOW_FILE_NAME);
        assert_eq!(WindowGeom::load(&p), None);
        let g = WindowGeom {
            x: -1500,
            y: 40,
            width: 1100.5,
            height: 720.0,
        };
        g.save(&p).unwrap();
        assert_eq!(WindowGeom::load(&p), Some(g));
        fs::write(&p, "not json").unwrap();
        assert_eq!(WindowGeom::load(&p), None, "a broken file is ignored");
    }

    #[test]
    fn the_window_file_sits_beside_the_other_settings() {
        let fb = tmp("winpaths");
        let paths = Paths::resolve(fb.join("sub").join("relink.exe"), fb.clone());
        assert_eq!(paths.window, fb.join(WINDOW_FILE_NAME));
        assert_eq!(paths.dir, fb);
    }
}

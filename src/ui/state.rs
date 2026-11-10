//! The window's state and the types the monitor publishes into it.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use rune::TextEdit;

use crate::config::Config;
use crate::diagnose::{HostKind, Report, Status};
use crate::t;

/// What the window asks the monitor to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    CheckNow,
    FixNow,
    /// Settings changed: reread the shared config.
    Reload,
    /// Mute notifications and automatic fixes for this many seconds.
    Pause(u64),
    /// End a pause early.
    Resume,
    /// Download and install the release the update check found.
    InstallUpdate,
}

/// The window's connection to the rest of the app.
pub struct Link {
    pub cmd: Sender<Cmd>,
    pub cfg: Arc<Mutex<Config>>,
    pub config_path: PathBuf,
    pub log_path: PathBuf,
    pub window_path: PathBuf,
    pub exe: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Overview,
    Connections,
    Hosts,
    Settings,
    Activity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Good,
    Warn,
}

/// One line of the activity log.
#[derive(Clone, Debug)]
pub struct Event {
    pub ts: u64,
    /// Local (year, month, day), for grouping by day.
    pub day: (u32, u32, u32),
    /// Local `HH:MM:SS`.
    pub time: String,
    pub text: String,
    pub level: Level,
}

impl Event {
    pub fn new(ts: u64, text: String, level: Level) -> Event {
        let (y, mo, d, h, mi, s) = crate::platform::local_parts(ts);
        Event {
            ts,
            day: (y, mo, d),
            time: format!("{h:02}:{mi:02}:{s:02}"),
            text,
            level,
        }
    }
}

/// Where a watched host came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    pub bookmark: bool,
    pub config: bool,
    pub learned: bool,
}

/// A watched host as the Hosts page shows it.
#[derive(Clone, Debug)]
pub struct HostRow {
    pub host: String,
    pub ips: Vec<std::net::Ipv4Addr>,
    /// The route found for each address (None: no route).
    pub routes: Vec<(std::net::Ipv4Addr, Option<crate::diagnose::Route>)>,
    /// `None` when the host did not resolve.
    pub kind: Option<HostKind>,
    pub sources: Sources,
}

/// What the monitor publishes after each check.
#[derive(Clone, Debug, Default)]
pub struct Published {
    pub report: Report,
    pub hosts: Vec<HostRow>,
    pub learned: usize,
    pub checked_at: u64,
    /// Whether the Linux root helper is installed and allowed.
    pub helper: bool,
}

pub const MAX_EVENTS: usize = 300;

#[derive(Default)]
pub struct Ui {
    pub page: Page,
    pub version: &'static str,
    pub published: Option<Published>,
    pub last_fix: Option<u64>,
    /// What the last fix restarted (process names), for the status page.
    pub last_fix_what: String,
    /// Until when (unix seconds) notifications are paused, if they are.
    pub paused_until: Option<u64>,
    /// A newer release than the running one, once the update check found it.
    pub update: Option<crate::update::Available>,
    pub update_stage: crate::update::Stage,
    pub events: VecDeque<Event>,
    pub cfg: Config,
    pub startup: bool,
    pub filter: TextEdit,
    /// Connections page filter (0 all, 1 stuck, 2 VPN, 3 internet) and activity filter (0 all, 1 warnings, 2 fixes).
    pub conn_filter: usize,
    pub event_filter: usize,
    /// Unix time of the last "copy diagnostics" press (0 = never), for the button's feedback.
    pub copied_at: u64,
    /// The host whose detail is open on the Hosts page.
    pub expanded_host: Option<String>,
    /// The window position was put back once; the geometry last written; and
    /// the one seen on the previous tick (it is saved once it stops changing).
    pub window_restored: bool,
    /// Ticks spent waiting for the window to exist, to look quickly only at
    /// the start (a tray-only start has no window for a long time).
    pub place_polls: u32,
    pub window_saved: Option<crate::config::WindowGeom>,
    pub window_seen: Option<crate::config::WindowGeom>,
    /// Inputs of the settings' list editors.
    pub add_domain: TextEdit,
    pub add_host: TextEdit,
    /// Until this unix time a second press of "Fix now" is confirmed (0 = not armed).
    pub fix_armed_until: u64,
    /// Unix time of the last "Check now" press, for the button's feedback (0 = never).
    pub check_pressed_at: u64,
    /// Unix seconds, refreshed every second while the window shows.
    pub now: u64,
    pub link: Option<Link>,
    /// The locale the view was last built in, to notice switches.
    pub locale: &'static str,
}

impl Ui {
    pub fn status(&self) -> Option<Status> {
        self.published.as_ref().map(|p| p.report.status())
    }

    pub fn push_event(&mut self, text: String, level: Level) {
        self.events
            .push_front(Event::new(crate::config::now_unix(), text, level));
        self.events.truncate(MAX_EVENTS);
    }

    pub(super) fn send(&self, cmd: Cmd) {
        if let Some(l) = &self.link {
            let _ = l.cmd.send(cmd);
        }
    }

    /// Saves a settings change, shares it with the monitor and applies the
    /// language at once.
    pub(super) fn change(&mut self, f: impl FnOnce(&mut Config)) {
        f(&mut self.cfg);
        crate::i18n::apply(&self.cfg.language);
        let Some(l) = &self.link else { return };
        *l.cfg.lock().unwrap() = self.cfg.clone();
        let saved = self.cfg.save(&l.config_path);
        let _ = l.cmd.send(Cmd::Reload);
        if let Err(e) = saved {
            let msg = t!("ev_config_error", error = e);
            self.push_event(msg, Level::Warn);
        }
    }
}

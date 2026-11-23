//! The background loop: collect, analyze, learn, publish to the window and
//! tray, then act on the engine's decisions. Runs on its own thread; the
//! window and tray reach it through `Cmd`s.

mod actions;
mod bookmark_cache;
mod check;
mod gate;
mod log;
mod names;
mod notify;
mod pace;
mod pause;
mod probe;
mod session;
mod settle;
mod unlock;
mod updates;
mod vpn;

use std::fs;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use rune::AppHandle;

use self::bookmark_cache::BookmarkCache;
use self::names::NameCache;
use self::pause::Pause;
use self::probe::Probe;
use self::settle::Settle;
use self::unlock::UnlockWatch;
use self::updates::Updates;
use crate::config::{self, Config, Paths, State};
use crate::diagnose::{Report, Status};
use crate::engine::Engine;
use crate::helper::Helper;
use crate::t;
use crate::tray::TrayHost;
use crate::ui::{Cmd, Level, Ui, short_status};

pub use self::log::{Log, load_history, parse_event};

pub struct Monitor {
    pub handle: AppHandle<Ui>,
    pub cfg: Arc<Mutex<Config>>,
    pub paths: Paths,
    pub tray: Option<Arc<TrayHost>>,
    pub log: Log,
    engine: Engine,
    settle: Settle,
    pause: Pause,
    names: NameCache,
    state: State,
    bookmarks: BookmarkCache,
    last_status: Option<Status>,
    last_report: Option<Report>,
    cfg_mtime: Option<SystemTime>,
    started: Instant,
    collect_error: Option<String>,
    helper: Helper,
    updates: Updates,
    unlock: UnlockWatch,
    /// When the VPN was last seen going down (None while it is up).
    vpn_down_since: Option<Instant>,
    /// Unix time of the last check, to notice a sleep in between.
    last_check_unix: Option<u64>,
    probe: Probe,
    /// The probe round in flight, if any.
    probe_rx: Option<std::sync::mpsc::Receiver<Option<probe::Round>>>,
    gate: gate::Schedule,
    /// The gate round in flight, if any.
    gate_rx: Option<std::sync::mpsc::Receiver<Vec<(String, crate::gate::Answer)>>>,
    /// A check ran before this one, so a session's end is seen when it happens.
    session_watched: bool,
    /// The session start already warned about.
    session_warned: Option<u64>,
}

impl Monitor {
    pub fn new(
        handle: AppHandle<Ui>,
        cfg: Arc<Mutex<Config>>,
        paths: Paths,
        tray: Option<Arc<TrayHost>>,
        log: Log,
    ) -> Monitor {
        let state = State::load(&paths.state);
        let cfg_mtime = fs::metadata(&paths.config).and_then(|m| m.modified()).ok();
        Monitor {
            handle,
            cfg,
            paths,
            tray,
            log,
            engine: Engine::default(),
            settle: Settle::new(),
            pause: Pause::default(),
            names: NameCache::default(),
            state,
            bookmarks: BookmarkCache::default(),
            last_status: None,
            last_report: None,
            cfg_mtime,
            started: Instant::now(),
            collect_error: None,
            helper: Helper::default(),
            updates: Updates::new(Instant::now()),
            unlock: UnlockWatch::default(),
            vpn_down_since: None,
            last_check_unix: None,
            probe: Probe::default(),
            probe_rx: None,
            gate: gate::Schedule::default(),
            gate_rx: None,
            session_watched: false,
            session_warned: None,
        }
    }

    fn mono(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn event(&self, text: String, level: Level) {
        self.log.event(&text, level);
        self.handle.update(move |s, _| s.push_event(text, level));
    }

    /// Runs until the window side goes away.
    pub fn run(mut self, rx: Receiver<Cmd>) {
        // When the next check is due. Commands that do not check (pause,
        // update) keep it, so they never push the next check back.
        let mut due = Instant::now();
        loop {
            match rx.recv_timeout(due.saturating_duration_since(Instant::now())) {
                Ok(Cmd::CheckNow) | Err(RecvTimeoutError::Timeout) => {}
                Ok(Cmd::FixNow) => {
                    let action = self.engine.user_fix(self.mono());
                    self.act(action);
                    due = Instant::now() + Duration::from_secs(3); // let Chromium bring the service back
                    continue;
                }
                Ok(Cmd::RestartFirefox) => {
                    self.restart_firefox();
                    due = Instant::now() + Duration::from_secs(10); // let it reconnect first
                    continue;
                }
                Ok(Cmd::Pause(secs)) => {
                    self.pause.start(secs, Instant::now());
                    self.pause_changed();
                    continue;
                }
                Ok(Cmd::Resume) => {
                    self.pause.clear();
                    self.pause_changed();
                    continue;
                }
                Ok(Cmd::InstallUpdate) => {
                    self.install_update();
                    continue;
                }
                Ok(Cmd::CheckUpdates) => {
                    self.check_updates_now();
                    continue;
                }
                Ok(Cmd::Reload) => {
                    self.cfg_mtime = fs::metadata(&self.paths.config)
                        .and_then(|m| m.modified())
                        .ok();
                    self.refresh_tray();
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.reload_if_edited();
            self.maybe_check_updates();
            self.check();
            let cfg = self.cfg.lock().unwrap().clone();
            let mut wait = match self.engine.next_deadline() {
                Some(t) => cfg
                    .interval()
                    .min(Duration::from_secs_f64((t - self.mono()).max(0.5))),
                None => cfg.interval(),
            };
            // The chosen interval is for a connected, settled VPN; while it is
            // down or its routes arrive, look more often.
            if let Some(cap) = self.pace() {
                wait = wait.min(cap);
            }
            due = Instant::now() + wait;
        }
    }

    /// The longest wait the VPN's state allows (per the last report).
    fn pace(&mut self) -> Option<Duration> {
        let report = self.last_report.as_ref();
        let up = report.is_some_and(|r| r.vpn.is_some());
        if up || report.is_none() {
            self.vpn_down_since = None;
        } else if self.vpn_down_since.is_none() {
            self.vpn_down_since = Some(Instant::now());
        }
        pace::cap(
            report.is_some(),
            up,
            report.is_some_and(|r| r.settling),
            self.vpn_down_since.map(|t| t.elapsed()),
        )
    }

    /// How old the last name lookup may be (per the last report's VPN state).
    fn name_refresh(&self) -> Option<std::time::Duration> {
        let r = self.last_report.as_ref();
        names::refresh(
            r.is_some_and(|r| r.vpn.is_some()),
            r.is_some_and(|r| r.settling),
        )
    }

    /// Picks up edits made to relink.json in an editor.
    fn reload_if_edited(&mut self) {
        let mtime = fs::metadata(&self.paths.config)
            .and_then(|m| m.modified())
            .ok();
        if mtime == self.cfg_mtime {
            return;
        }
        self.cfg_mtime = mtime;
        let (cfg, err) = Config::load(&self.paths.config);
        if let Some(e) = err {
            self.event(t!("ev_config_error", error = e), Level::Warn);
            return;
        }
        crate::i18n::apply(&cfg.language);
        *self.cfg.lock().unwrap() = cfg.clone();
        self.handle.update(move |s, _| s.cfg = cfg);
        self.event(t!("ev_config_reloaded"), Level::Info);
        self.refresh_tray();
    }

    /// Tells the window, the notifier and the tray about the pause state.
    fn pause_changed(&self) {
        let now = Instant::now();
        notify::set_paused(self.pause.active(now));
        let until = self.pause.until_unix(now, config::now_unix());
        self.handle.update(move |s, _| s.paused_until = until);
        self.refresh_tray();
    }

    fn refresh_tray(&self) {
        if let Some(tray) = &self.tray {
            let paused = self.pause.active(Instant::now());
            let mut tooltip = format!("Re-Link — {}", short_status(self.last_report.as_ref()));
            let up = self.last_report.as_ref().is_some_and(|r| r.vpn.is_some());
            if let Some(since) = self.state.vpn_up_since.filter(|_| up) {
                let d = crate::ui::duration_text(config::now_unix().saturating_sub(since));
                tooltip.push_str(&format!(" · {}", t!("vpn_up_for", duration = d)));
            }
            if paused {
                tooltip.push_str(&format!(" · {}", t!("paused_short")));
            }
            tray.set(self.last_status, tooltip, paused);
        }
    }
}

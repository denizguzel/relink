//! The background loop: collect, analyze, learn, publish to the window and
//! tray, then act on the engine's decisions. Runs on its own thread; the
//! window and tray reach it through `Cmd`s.

mod actions;
mod bookmark_cache;
mod check;
mod log;
mod names;
mod notify;
mod pause;
mod settle;
mod updates;

use std::fs;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use rune::AppHandle;

use self::bookmark_cache::BookmarkCache;
use self::names::NameCache;
use self::pause::Pause;
use self::settle::Settle;
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
        let mut wait = Duration::ZERO;
        loop {
            match rx.recv_timeout(wait) {
                Ok(Cmd::CheckNow) | Err(RecvTimeoutError::Timeout) => {}
                Ok(Cmd::FixNow) => {
                    let action = self.engine.user_fix(self.mono());
                    self.act(action);
                    wait = Duration::from_secs(3); // let Chromium bring the service back
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
            wait = match self.engine.next_deadline() {
                Some(t) => cfg
                    .interval()
                    .min(Duration::from_secs_f64((t - self.mono()).max(0.5))),
                None => cfg.interval(),
            };
            // The chosen interval is for a connected, settled VPN; while it is
            // down or its routes arrive, look every second.
            if self.fast_mode(&cfg) {
                wait = wait.min(Duration::from_secs(1));
            }
        }
    }

    /// Whether the VPN is down or still installing its routes (per the last report).
    fn fast_mode(&self, _cfg: &Config) -> bool {
        self.last_report
            .as_ref()
            .is_none_or(|r| r.vpn.is_none() || r.settling)
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
            if paused {
                tooltip.push_str(&format!(" · {}", t!("paused_short")));
            }
            tray.set(self.last_status, tooltip, paused);
        }
    }
}

//! The VPN beyond its routes: whether it carries traffic, how long it has been
//! up (and when the session will likely end), and what happened to it while
//! the user was away. The decisions live in `probe`, `session` and `unlock`;
//! this is where the monitor acts on them.

use std::sync::mpsc::TryRecvError;

use super::Monitor;
use super::notify::notify;
use super::probe::{self, Change};
use super::session::{self, Seen};
use super::unlock::{self, UnlockMode};
use crate::config::{self, Config};
use crate::diagnose::Report;
use crate::t;
use crate::ui::Level;

impl Monitor {
    /// Everything about the VPN this check adds to `report`, before it is
    /// published: the tunnel probe's verdict.
    pub(super) fn watch_vpn(&mut self, report: &mut Report, cfg: &Config) {
        let now = config::now_unix();
        let woke = unlock::slept(self.last_check_unix, now, cfg.interval());
        self.last_check_unix = Some(now);
        self.probe_tunnel(report, cfg);
        self.follow_session(report, woke, now, cfg);
        let locked = crate::platform::session_locked();
        if self.unlock.observe(locked, woke, report.vpn.is_some()) {
            self.vpn_lost_while_away(&cfg.vpn_after_unlock);
        }
    }

    /// Collects a finished probe round and starts the next when due.
    fn probe_tunnel(&mut self, report: &mut Report, cfg: &Config) {
        let now = self.mono();
        let usable = report.vpn.is_some() && !report.settling;
        if let Some(rx) = &self.probe_rx {
            match rx.try_recv() {
                Ok(round) => {
                    self.probe_rx = None;
                    // A round that ended after the VPN went away says nothing.
                    if usable && self.probe.record(round, now) == Some(Change::Died) {
                        self.tunnel_died();
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.probe_rx = None,
            }
        }
        let due = self.probe.due(report, cfg.check_tunnel, now);
        if due && self.probe_rx.is_none() {
            let targets = probe::targets(report);
            let (tx, rx) = std::sync::mpsc::channel();
            self.probe_rx = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(probe::run(&targets));
            });
        }
        report.tunnel_dead = self.probe.dead();
    }

    fn tunnel_died(&self) {
        let client = crate::platform::vpn_client_available();
        notify(
            "vpn",
            &t!("notify_tunnel_dead_title"),
            &if client {
                t!("notify_tunnel_dead_body")
            } else {
                t!("notify_tunnel_dead_body_no_client")
            },
            Some(if client { "connect-vpn" } else { "open" }),
        );
    }

    /// Records the session's start and end, and warns before its likely end.
    fn follow_session(&mut self, report: &Report, woke: bool, now: u64, cfg: &Config) {
        let seen = Seen {
            vpn_up: report.vpn.is_some(),
            now,
            watched: self.session_watched && !woke,
        };
        self.session_watched = true;
        let st = &mut self.state;
        if session::follow(
            &mut st.vpn_up_since,
            &mut st.vpn_seen_at,
            &mut st.vpn_sessions,
            seen,
        ) && let Err(e) = st.save(&self.paths.state)
        {
            self.log.line(&format!("save state: {e}"));
        }
        let since = self.state.vpn_up_since;
        let limit = session::limit(&self.state.vpn_sessions);
        if cfg.session_warning
            && since.is_some()
            && self.session_warned != since
            && session::warn_due(since, limit, now)
        {
            self.session_warned = since;
            let hours = limit.unwrap_or(0).div_ceil(1800) as f64 / 2.0;
            let hours = format!("{hours}");
            self.event(t!("ev_session_ending", hours = hours.clone()), Level::Info);
            notify(
                "session",
                &t!("notify_session_title"),
                &t!("notify_session_body", hours = hours),
                None,
            );
        }
    }

    /// The user is back (unlocked, or the computer woke) and the VPN, up
    /// before they left, is gone: open FortiClient or say so, as the setting
    /// asks. A pause keeps both quiet.
    fn vpn_lost_while_away(&mut self, setting: &str) {
        let mode = UnlockMode::parse(setting);
        if mode == UnlockMode::Off {
            return;
        }
        self.event(t!("ev_vpn_lost_locked"), Level::Info);
        if self.pause.active(std::time::Instant::now()) {
            return;
        }
        let client = crate::platform::vpn_client_available();
        if mode == UnlockMode::Open && client && crate::platform::open_vpn_client() {
            return;
        }
        notify(
            "vpn",
            &t!("notify_vpn_lost_title"),
            &if client {
                t!("notify_vpn_lost_body")
            } else {
                t!("notify_vpn_lost_body_no_client")
            },
            Some(if client { "connect-vpn" } else { "open" }),
        );
    }
}

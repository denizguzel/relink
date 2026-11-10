//! One check: collect, analyze, learn, publish, and hand the engine's
//! decisions to `act`.

use super::Monitor;
use std::time::Instant;

use crate::config;
use crate::diagnose::{self, Status};
use crate::engine;
use crate::ui::{HostRow, Level, Published, Sources, status_text};
use crate::{net, t};
use std::collections::BTreeSet;

impl Monitor {
    pub(super) fn check(&mut self) {
        let cfg = self.cfg.lock().unwrap().clone();
        super::notify::set_level(&cfg.notifications);
        let bookmark_hosts = self.bookmarks.hosts(&cfg);
        let watched = config::merge(&[&cfg.hosts, &bookmark_hosts]);
        // Without the VPN, or while its routes arrive, checks run every
        // second; resolving 50 names that often is pointless, so reuse the
        // last answer for ten seconds.
        let fast = self.fast_mode(&cfg);
        let resolve_now = self.names.due(fast);
        let mut snap = match net::collect(&cfg.browsers, if resolve_now { &watched } else { &[] }) {
            Ok(mut s) => {
                self.collect_error = None;
                if resolve_now {
                    self.names.store(&s.hosts);
                } else {
                    s.hosts = self.names.hosts();
                }
                s
            }
            Err(e) => {
                if self.collect_error.as_deref() != Some(&e) {
                    self.event(e.clone(), Level::Warn);
                    self.collect_error = Some(e);
                }
                return;
            }
        };
        snap.required = self.state.required(&cfg.hosts);
        let mut report = diagnose::analyze(&snap, &cfg.vpn_adapter_keywords, net::best_route);
        self.settle.observe(&mut report);

        if self.state.learn(&report.vpn_hosts, config::now_unix())
            && let Err(e) = self.state.save(&self.paths.state)
        {
            self.log.line(&format!("save state: {e}"));
        }

        let bookmark_set: BTreeSet<&String> = bookmark_hosts.iter().collect();
        let kinds: std::collections::HashMap<&str, diagnose::HostKind> =
            report.hosts.iter().map(|h| (h.0.as_str(), h.2)).collect();
        let hosts: Vec<HostRow> = watched
            .iter()
            .map(|h| {
                let ips = snap.hosts.get(h).cloned().unwrap_or_default();
                HostRow {
                    routes: ips
                        .iter()
                        .map(|ip| (*ip, report.route_of.get(ip).copied().flatten()))
                        .collect(),
                    host: h.clone(),
                    ips,
                    kind: kinds.get(h.as_str()).copied(),
                    sources: Sources {
                        bookmark: bookmark_set.contains(h),
                        config: cfg.hosts.contains(h),
                        learned: self.state.vpn_hosts.contains_key(h),
                    },
                }
            })
            .collect();

        let status = report.status();
        if self.last_status != Some(status) {
            self.event(
                t!(
                    "ev_status",
                    status = status_text(Some(status), Some(&report))
                ),
                match status {
                    Status::Ok => Level::Good,
                    Status::VpnDown | Status::Settling => Level::Info,
                    _ => Level::Warn,
                },
            );
            let mut detail = String::new();
            for st in &report.stuck {
                detail.push_str(&format!(
                    "\n  stuck {} pid={} {} -> {}:{} (route /{} if {})",
                    st.conn.process,
                    st.conn.pid,
                    st.conn.local,
                    st.conn.remote,
                    st.conn.remote_port,
                    st.route.prefix_len,
                    st.route.if_index
                ));
            }
            for m in &report.missing {
                detail.push_str(&format!(
                    "\n  missing {} {} via if {}",
                    m.host, m.ip, m.route.if_index
                ));
            }
            if !detail.is_empty() {
                self.log.line(detail.trim_start());
            }
        }
        self.last_status = Some(status);
        self.last_report = Some(report.clone());
        self.refresh_tray();

        if self.pause.expire(Instant::now()) {
            self.pause_changed();
        }
        let helper = self.helper.available();
        let actions = self
            .engine
            .on_report(&report, cfg.auto_fix, helper, self.mono());
        let actions = engine::filter_paused(actions, self.pause.active(Instant::now()));
        let published = Published {
            report,
            hosts,
            learned: self.state.vpn_hosts.len(),
            checked_at: config::now_unix(),
            helper,
        };
        self.handle
            .update(move |s, _| s.published = Some(published));
        for a in actions {
            self.act(a);
        }
    }
}

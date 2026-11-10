//! What the engine decided: fix, notify, add routes.

use super::Monitor;
use super::notify::notify;
use crate::engine::{Action, FixReason};
use crate::ui::Level;
use crate::{config, net, t};

impl Monitor {
    pub(super) fn act(&mut self, action: Action) {
        match action {
            Action::Fix { automatic, reason } => {
                // With the root helper, close just the stuck sockets: nothing
                // restarts and it works for Firefox too. A fix the user asks
                // for also restarts Chromium's network service, which clears
                // its DNS cache as well.
                let mut closed = 0;
                if self.helper.available() {
                    for st in self
                        .last_report
                        .as_ref()
                        .map(|r| r.stuck.clone())
                        .unwrap_or_default()
                    {
                        let c = &st.conn;
                        match self.helper.kill_socket(
                            c.local,
                            c.local_port,
                            c.remote,
                            c.remote_port,
                        ) {
                            Ok(_) => closed += 1,
                            Err(e) => self.log.line(&format!(
                                "kill-socket {}:{} -> {}:{}: {e}",
                                c.local, c.local_port, c.remote, c.remote_port
                            )),
                        }
                    }
                    if closed > 0 {
                        self.event(t!("ev_sockets_closed", count = closed), Level::Good);
                    }
                }
                let restart = !(closed > 0 && automatic);
                // Restart the network process of the browsers that have stuck
                // connections; after a VPN reconnect (or when nothing is stuck
                // and the user asks anyway) every browser's, to drop DNS caches.
                let stuck_names = self
                    .last_report
                    .as_ref()
                    .map(|r| r.stuck_processes())
                    .unwrap_or_default();
                let mut browsers = net::watched(&self.cfg.lock().unwrap().browsers);
                if reason != FixReason::VpnReconnected && !stuck_names.is_empty() {
                    browsers.retain(|b| stuck_names.iter().any(|s| s.eq_ignore_ascii_case(b)));
                }
                let (names, errs) = if restart {
                    net::restart_network_services(&browsers)
                } else {
                    (Vec::new(), vec![])
                };
                let now = config::now_unix();
                let n = names.len();
                let names = names.join(", ");
                let what = names.clone();
                self.handle.update(move |s, _| {
                    s.last_fix = Some(now);
                    s.last_fix_what = what;
                });
                if n > 0 {
                    let text = match reason {
                        FixReason::User => t!("ev_fixed", names = names),
                        FixReason::Stuck => t!("ev_fixed_auto", names = names),
                        FixReason::VpnReconnected => t!("ev_fixed_reconnect", names = names),
                    };
                    self.event(text, Level::Good);
                    notify(
                        "fixed",
                        &t!("notify_fixed_title"),
                        &t!("notify_fixed_body"),
                        None,
                    );
                } else if closed > 0 {
                    notify(
                        "fixed",
                        &t!("notify_fixed_title"),
                        &t!("notify_sockets_body"),
                        None,
                    );
                } else if let Some(e) = errs.first() {
                    self.event(t!("ev_fix_failed", error = e), Level::Warn);
                } else if !automatic {
                    match stuck_names.first() {
                        // The stuck sockets live in the browser's main process.
                        Some(p) => {
                            self.event(t!("ev_no_network_process", process = p), Level::Warn)
                        }
                        None => self.event(t!("ev_nothing_to_fix"), Level::Info),
                    }
                }
                for e in errs.iter().skip(1) {
                    self.log.line(e);
                }
            }
            Action::NotifyStuck { processes } => {
                let title = t!("notify_stuck_title");
                if processes.iter().all(|p| crate::diagnose::is_firefox(p))
                    && !self.helper.available()
                {
                    // Its sockets live in the main process: nothing Re-Link can restart.
                    notify(
                        "stuck",
                        &title,
                        &t!(
                            "notify_stuck_firefox_body",
                            processes = processes.join(", ")
                        ),
                        None,
                    );
                } else {
                    notify(
                        "stuck",
                        &title,
                        &t!("notify_stuck_body", processes = processes.join(", ")),
                        Some("fix"),
                    );
                }
                self.event(t!("ev_notified", title = title), Level::Warn);
            }
            Action::AddRoutes { routes } => {
                let mut added = Vec::new();
                for (host, ip) in routes {
                    match self.helper.route_add(ip) {
                        Ok(_) => {
                            self.event(t!("ev_route_added", host = host, ip = ip), Level::Good);
                            if !added.contains(&host) {
                                added.push(host);
                            }
                        }
                        Err(e) => {
                            self.event(t!("ev_route_failed", host = host, error = e), Level::Warn)
                        }
                    }
                }
                if !added.is_empty() {
                    notify(
                        "route",
                        &t!("notify_route_title"),
                        &t!("notify_route_body", hosts = added.join(", ")),
                        None,
                    );
                }
            }
            Action::NotifyMissing { hosts } => {
                let title = t!("notify_missing_title");
                notify(
                    "missing",
                    &title,
                    &t!("notify_missing_body", hosts = hosts.join(", ")),
                    Some("open"),
                );
                self.event(t!("ev_notified", title = title), Level::Warn);
            }
        }
    }
}

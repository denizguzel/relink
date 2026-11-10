//! What to do after each check: notify, fix, or stay quiet. Pure and
//! time-injected so every policy is unit-tested.

use std::collections::HashMap;
use std::time::Duration;

use crate::diagnose::Report;

/// Something the monitor should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Restart the browsers' network service. `automatic` is false when the
    /// user asked for it.
    Fix { automatic: bool, reason: FixReason },
    /// Tell the user about stuck connections; a click should fix them.
    NotifyStuck { processes: Vec<String> },
    /// Tell the user to reconnect the VPN.
    NotifyMissing { hosts: Vec<String> },
    /// Route these addresses through the VPN tunnel ourselves (Linux with
    /// the root helper), so nobody has to reconnect.
    AddRoutes {
        routes: Vec<(String, std::net::Ipv4Addr)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixReason {
    Stuck,
    VpnReconnected,
    User,
}

/// How long the same stuck set stays quiet after being announced.
pub const STUCK_REPEAT: Duration = Duration::from_secs(10 * 60);
/// How long the same missing-route set stays quiet after being announced.
pub const MISSING_REPEAT: Duration = Duration::from_secs(30 * 60);
/// Minimum gap between two automatic fixes.
pub const AUTO_FIX_GAP: Duration = Duration::from_secs(30);
/// Minimum gap between two attempts to add the same missing routes.
pub const ROUTE_RETRY: Duration = Duration::from_secs(120);
/// FortiClient needs a moment after connecting to install its routes.
pub const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Drops what would interrupt the user while notifications are paused:
/// notices and automatic fixes. Routes are still added (silent and useful), and
/// a fix the user asked for is never dropped.
pub fn filter_paused(actions: Vec<Action>, paused: bool) -> Vec<Action> {
    if !paused {
        return actions;
    }
    actions
        .into_iter()
        .filter(|a| {
            !matches!(
                a,
                Action::NotifyStuck { .. }
                    | Action::NotifyMissing { .. }
                    | Action::Fix {
                        automatic: true,
                        ..
                    }
            )
        })
        .collect()
}

/// Decision state carried between checks. Times are seconds on any
/// monotonic clock.
#[derive(Debug, Default)]
pub struct Engine {
    notified: HashMap<String, f64>,
    last_fix: Option<f64>,
    vpn_was_up: bool,
    had_missing: bool,
    fix_at: Option<f64>,
}

impl Engine {
    /// When a scheduled fix is due, so the monitor can wake for it.
    pub fn next_deadline(&self) -> Option<f64> {
        self.fix_at
    }

    /// Decides what to do about `r` at time `now`. `can_route` says whether
    /// missing routes can be added locally instead of asking for a reconnect.
    pub fn on_report(
        &mut self,
        r: &Report,
        auto_fix: bool,
        can_route: bool,
        now: f64,
    ) -> Vec<Action> {
        let mut out = Vec::new();
        // Right after the VPN connects its routes are still being installed.
        let waiting = r.waiting_for_routes();
        let missing = !waiting && !r.missing.is_empty();
        if can_route
            && missing
            && self.once(format!("route:{}", r.missing_signature()), ROUTE_RETRY, now)
        {
            out.push(Action::AddRoutes {
                routes: r.missing.iter().map(|m| (m.host.clone(), m.ip)).collect(),
            });
        }
        let vpn_up = r.vpn.is_some();

        // The VPN came back after we asked the user to reconnect: give
        // FortiClient a few seconds to install its routes, then clear the
        // browser's sockets and DNS cache.
        if vpn_up && !self.vpn_was_up && self.had_missing {
            self.fix_at = Some(now + RECONNECT_DELAY.as_secs_f64());
            self.had_missing = false;
        }
        self.vpn_was_up = vpn_up;
        if missing && !can_route {
            self.had_missing = true;
        }

        if self.fix_at.is_some_and(|t| now >= t) {
            self.fix_at = None;
            self.fixed(now);
            return vec![Action::Fix {
                automatic: true,
                reason: FixReason::VpnReconnected,
            }];
        }

        if !r.stuck.is_empty() && !waiting {
            if auto_fix
                && self
                    .last_fix
                    .is_none_or(|t| now - t > AUTO_FIX_GAP.as_secs_f64())
            {
                self.fixed(now);
                out.push(Action::Fix {
                    automatic: true,
                    reason: FixReason::Stuck,
                });
                return out;
            }
            if !auto_fix && self.once(format!("stuck:{}", r.stuck_signature()), STUCK_REPEAT, now) {
                out.push(Action::NotifyStuck {
                    processes: r.stuck_processes(),
                });
            }
        }
        if !can_route
            && missing
            && self.once(
                format!("missing:{}", r.missing_signature()),
                MISSING_REPEAT,
                now,
            )
        {
            out.push(Action::NotifyMissing {
                hosts: r.missing_hosts(),
            });
        }
        out
    }

    /// Records a fix the user asked for.
    pub fn user_fix(&mut self, now: f64) -> Action {
        self.fixed(now);
        Action::Fix {
            automatic: false,
            reason: FixReason::User,
        }
    }

    fn fixed(&mut self, now: f64) {
        self.last_fix = Some(now);
        // A fix changes the picture; announce whatever is left afresh.
        self.notified.clear();
    }

    fn once(&mut self, sig: String, every: Duration, now: f64) -> bool {
        match self.notified.get(&sig) {
            Some(&t) if now - t < every.as_secs_f64() => false,
            _ => {
                self.notified.insert(sig, now);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::{Adapter, Conn, Missing, Route, Stuck};
    use std::net::Ipv4Addr;

    fn vpn() -> Option<Adapter> {
        Some(Adapter {
            index: 20,
            description: "Fortinet SSL VPN".into(),
            up: true,
            addrs: vec![Ipv4Addr::new(10, 0, 0, 2)],
        })
    }
    fn route() -> Route {
        Route {
            if_index: 20,
            prefix_len: 32,
            source: Some(Ipv4Addr::new(10, 0, 0, 2)),
        }
    }
    fn stuck(ip: [u8; 4]) -> Report {
        Report {
            vpn: vpn(),
            stuck: vec![Stuck {
                conn: Conn {
                    pid: 1,
                    process: "chrome.exe".into(),
                    local: Ipv4Addr::new(192, 168, 1, 2),
                    local_port: 50000,
                    remote: ip.into(),
                    remote_port: 443,
                },
                route: route(),
            }],
            ..Default::default()
        }
    }
    fn missing() -> Report {
        Report {
            vpn: vpn(),
            missing: vec![Missing {
                host: "a.example".into(),
                ip: Ipv4Addr::new(1, 2, 3, 4),
                route: route(),
            }],
            ..Default::default()
        }
    }
    fn ok() -> Report {
        Report {
            vpn: vpn(),
            ..Default::default()
        }
    }
    fn down() -> Report {
        Report::default()
    }

    #[test]
    fn stuck_is_announced_once_per_set_and_again_after_the_quiet_period() {
        let mut e = Engine::default();
        assert_eq!(
            e.on_report(&stuck([1, 1, 1, 1]), false, false, 0.0),
            vec![Action::NotifyStuck {
                processes: vec!["chrome.exe".into()]
            }]
        );
        assert!(
            e.on_report(&stuck([1, 1, 1, 1]), false, false, 60.0)
                .is_empty()
        );
        assert_eq!(
            e.on_report(&stuck([2, 2, 2, 2]), false, false, 61.0).len(),
            1,
            "a new set is announced"
        );
        assert_eq!(
            e.on_report(&stuck([1, 1, 1, 1]), false, false, 601.0).len(),
            1,
            "after ten minutes again"
        );
    }

    #[test]
    fn auto_fix_fixes_but_not_more_than_every_30_seconds() {
        let mut e = Engine::default();
        assert!(matches!(
            e.on_report(&stuck([1, 1, 1, 1]), true, false, 0.0)[..],
            [Action::Fix {
                automatic: true,
                reason: FixReason::Stuck
            }]
        ));
        assert!(
            e.on_report(&stuck([1, 1, 1, 1]), true, false, 10.0)
                .is_empty(),
            "no fix storm, no notification either"
        );
        assert!(matches!(
            e.on_report(&stuck([1, 1, 1, 1]), true, false, 31.0)[..],
            [Action::Fix { .. }]
        ));
    }

    #[test]
    fn missing_route_notifies_and_reconnect_triggers_a_delayed_fix() {
        let mut e = Engine::default();
        assert_eq!(
            e.on_report(&missing(), false, false, 0.0),
            vec![Action::NotifyMissing {
                hosts: vec!["a.example".into()]
            }]
        );
        assert!(e.on_report(&missing(), false, false, 15.0).is_empty());
        assert!(
            e.on_report(&down(), false, false, 30.0).is_empty(),
            "user disconnects"
        );
        assert!(
            e.on_report(&ok(), false, false, 40.0).is_empty(),
            "reconnected: fix scheduled, not yet due"
        );
        assert_eq!(e.next_deadline(), Some(45.0));
        assert_eq!(
            e.on_report(&ok(), false, false, 45.0),
            vec![Action::Fix {
                automatic: true,
                reason: FixReason::VpnReconnected
            }]
        );
        assert!(
            e.on_report(&ok(), false, false, 60.0).is_empty(),
            "only once"
        );
    }

    #[test]
    fn with_the_helper_missing_routes_are_added_not_announced() {
        let mut e = Engine::default();
        let a = e.on_report(&missing(), false, true, 0.0);
        assert_eq!(
            a,
            vec![Action::AddRoutes {
                routes: vec![("a.example".into(), Ipv4Addr::new(1, 2, 3, 4))]
            }]
        );
        assert!(
            e.on_report(&missing(), false, true, 30.0).is_empty(),
            "no retry storm"
        );
        assert_eq!(
            e.on_report(&missing(), false, true, 121.0).len(),
            1,
            "retried after two minutes"
        );
        assert!(e.on_report(&down(), false, true, 130.0).is_empty());
        assert!(e.on_report(&ok(), false, true, 140.0).is_empty());
        assert_eq!(e.next_deadline(), None, "no reconnect fix was promised");
    }

    #[test]
    fn a_plain_vpn_connect_does_not_fix() {
        let mut e = Engine::default();
        assert!(e.on_report(&down(), false, false, 0.0).is_empty());
        assert!(e.on_report(&ok(), false, false, 10.0).is_empty());
        assert_eq!(e.next_deadline(), None);
    }

    #[test]
    fn a_user_fix_resets_announcements() {
        let mut e = Engine::default();
        assert_eq!(
            e.on_report(&stuck([1, 1, 1, 1]), false, false, 0.0).len(),
            1
        );
        assert!(matches!(
            e.user_fix(5.0),
            Action::Fix {
                automatic: false,
                reason: FixReason::User
            }
        ));
        assert_eq!(
            e.on_report(&stuck([1, 1, 1, 1]), false, false, 10.0).len(),
            1,
            "still stuck after the fix: say so"
        );
    }

    #[test]
    fn missing_routes_are_ignored_while_the_vpn_settles() {
        let mut e = Engine::default();
        let mut r = Report {
            vpn: vpn(),
            settling: true,
            ..Default::default()
        };
        r.missing.push(Missing {
            host: "a.example".into(),
            ip: Ipv4Addr::new(1, 2, 3, 4),
            route: route(),
        });
        assert!(e.on_report(&r, false, false, 0.0).is_empty());
        assert_eq!(r.status(), crate::diagnose::Status::Settling);
        r.settling = false;
        assert_eq!(r.status(), crate::diagnose::Status::MissingRoute);
        assert_eq!(
            e.on_report(&r, false, false, 10.0),
            vec![Action::NotifyMissing {
                hosts: vec!["a.example".into()]
            }]
        );
    }

    #[test]
    fn stuck_connections_wait_while_routes_are_arriving() {
        let mut e = Engine::default();
        let mut r = stuck([52, 50, 137, 113]);
        r.settling = true;
        r.missing.push(Missing {
            host: "a.example".into(),
            ip: Ipv4Addr::new(1, 2, 3, 4),
            route: route(),
        });
        assert!(
            e.on_report(&r, true, false, 0.0).is_empty(),
            "no fix or notice while routes arrive"
        );
        r.missing.clear();
        r.settling = false;
        assert!(
            !e.on_report(&r, true, false, 40.0).is_empty(),
            "once the wait is over, the stuck one is handled"
        );
    }

    #[test]
    fn pausing_drops_notices_and_automatic_fixes_only() {
        let actions = vec![
            Action::NotifyStuck {
                processes: vec!["chrome.exe".into()],
            },
            Action::NotifyMissing {
                hosts: vec!["a.example".into()],
            },
            Action::Fix {
                automatic: true,
                reason: FixReason::Stuck,
            },
            Action::Fix {
                automatic: false,
                reason: FixReason::User,
            },
            Action::AddRoutes {
                routes: vec![("a.example".into(), Ipv4Addr::new(1, 2, 3, 4))],
            },
        ];
        let kept = filter_paused(actions.clone(), true);
        assert_eq!(kept, actions[3..].to_vec());
        assert_eq!(filter_paused(actions.clone(), false), actions);
    }
}

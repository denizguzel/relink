//! Waiting for the VPN's routes. A VPN that connects while Re-Link watches
//! needs a while to install its routes; until then missing routes are no news.
//! Waiting lasts at least `MIN_SETTLE` (`LATE_WAVE` while hosts are still
//! missing), goes on while the missing set keeps changing (no change for
//! `QUIET` ends it) and never exceeds `MAX_SETTLE`.
//! The same wait starts when hosts suddenly go missing while the VPN stays up
//! (FortiClient reinstalls its routes in waves without the adapter dropping).
//! Once a wait has ended, routes that arrive later do not reopen it.

use std::time::{Duration, Instant};

use crate::diagnose::Report;

const MIN_SETTLE: Duration = Duration::from_secs(20);
/// FortiClient's last wave of routes (often the hosts whose addresses changed
/// when DNS switched to the company's servers) arrives 30–45 s after
/// connecting, so a missing host is no news before this.
const LATE_WAVE: Duration = Duration::from_secs(50);
const QUIET: Duration = Duration::from_secs(20);
const MAX_SETTLE: Duration = Duration::from_secs(150);

pub(super) struct Settle {
    /// Whether the VPN was up at the previous check (None before the first).
    was_up: Option<bool>,
    /// When the current wait started (None when not waiting).
    since: Option<Instant>,
    /// When the missing set last changed during the wait.
    activity: Instant,
    /// The missing set at the previous check (`Report::missing_signature`).
    /// A wait starts when it goes from empty to non-empty; any change while
    /// waiting (a route arrived, a name now resolves to other addresses)
    /// shows the VPN is still settling.
    missing_before: String,
}

impl Settle {
    pub(super) fn new() -> Settle {
        Settle {
            was_up: None,
            since: None,
            activity: Instant::now(),
            missing_before: String::new(),
        }
    }

    /// Marks `report` as settling while the routes are still arriving.
    pub(super) fn observe(&mut self, report: &mut Report) {
        self.observe_at(report, Instant::now());
    }

    /// [`Settle::observe`] at a given time, so the rules can be tested.
    fn observe_at(&mut self, report: &mut Report, now: Instant) {
        let up = report.vpn.is_some();
        let missing = report.missing_signature();
        let connected = up && self.was_up == Some(false);
        // Routes vanished while the VPN stayed up: they are being reinstalled.
        let vanished =
            up && self.since.is_none() && self.missing_before.is_empty() && !missing.is_empty();
        if connected || vanished {
            self.since = Some(now);
            self.activity = now;
        } else if !up {
            self.since = None;
        } else if missing != self.missing_before {
            self.activity = now;
        }
        self.was_up = Some(up);
        self.missing_before = missing;
        let Some(since) = self.since else {
            return;
        };
        let elapsed = now.duration_since(since);
        let min = if report.missing.is_empty() {
            MIN_SETTLE
        } else {
            LATE_WAVE
        };
        report.settling =
            elapsed < MAX_SETTLE && (elapsed < min || now.duration_since(self.activity) < QUIET);
        if !report.settling {
            self.since = None;
        }
    }
}
#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::diagnose::{Adapter, Missing, Route};

    /// A report with the VPN up or down and `missing` hosts off the VPN; `wave`
    /// picks their addresses, so the same count can stand for other addresses.
    fn report(vpn: bool, missing: usize, wave: u8) -> Report {
        let mut r = Report::default();
        if vpn {
            r.vpn = Some(Adapter {
                index: 20,
                description: "Fortinet SSL VPN".into(),
                up: true,
                addrs: vec![Ipv4Addr::new(10, 0, 0, 2)],
            });
        }
        for i in 0..missing {
            r.missing.push(Missing {
                host: format!("h{i}.example"),
                ip: Ipv4Addr::new(1, 1, wave, i as u8),
                route: Route {
                    if_index: 7,
                    prefix_len: 0,
                    source: None,
                },
            });
        }
        r
    }

    /// Runs the tracker over (seconds, vpn up, missing hosts, address wave)
    /// steps and returns `settling` at each.
    fn run_waves(steps: &[(u64, bool, usize, u8)]) -> Vec<bool> {
        let (t0, mut s) = (Instant::now(), Settle::new());
        steps
            .iter()
            .map(|&(secs, vpn, missing, wave)| {
                let mut r = report(vpn, missing, wave);
                s.observe_at(&mut r, t0 + Duration::from_secs(secs));
                r.settling
            })
            .collect()
    }

    /// [`run_waves`] with addresses that never change.
    fn run(steps: &[(u64, bool, usize)]) -> Vec<bool> {
        let steps: Vec<_> = steps.iter().map(|&(t, v, m)| (t, v, m, 0)).collect();
        run_waves(&steps)
    }

    #[test]
    fn a_vpn_that_was_already_up_at_startup_with_nothing_missing_is_not_settling() {
        assert_eq!(run(&[(0, true, 0), (5, true, 0)]), [false, false]);
    }

    #[test]
    fn routes_vanishing_while_the_vpn_stays_up_start_a_wait() {
        let got = run(&[
            (0, true, 0),
            (30, true, 0),
            (40, true, 6),  // the routes went away
            (61, true, 6),  // no change, but the late wave may still come
            (91, true, 6),  // 51 s and still missing: a real problem
            (95, true, 0),  // all back later: healthy at once
            (100, true, 2), // and again later
        ]);
        assert_eq!(got, [false, false, true, true, false, false, true]);
    }

    #[test]
    fn waits_while_routes_keep_arriving_then_stops_when_they_stall() {
        let got = run(&[
            (0, false, 0),
            (1, true, 10), // VPN just came up
            (10, true, 7),
            (25, true, 4), // still shrinking
            (40, true, 4), // 15 s without change: still within QUIET
            (55, true, 4), // 30 s without change, past the late wave: give up
        ]);
        assert_eq!(got, [false, true, true, true, true, false]);
    }

    #[test]
    fn names_resolving_to_new_addresses_count_as_progress() {
        let got = run_waves(&[
            (0, false, 0, 0),
            (1, true, 1, 0),
            (5, true, 33, 0), // DNS switched to the company's servers: more missing
            (40, true, 3, 0),
            (45, true, 3, 1), // same hosts, other addresses: still moving
            (60, true, 3, 1),
            (66, true, 3, 1), // 21 s without change
        ]);
        assert_eq!(got, [false, true, true, true, true, true, false]);
    }

    #[test]
    fn never_waits_past_the_maximum() {
        let got = run(&[
            (0, false, 0),
            (1, true, 100),
            (60, true, 90),
            (120, true, 80),
            (155, true, 70),
        ]);
        assert_eq!(got, [false, true, true, true, false]);
    }

    #[test]
    fn with_nothing_missing_the_short_minimum_applies() {
        let got = run(&[(0, false, 0), (1, true, 0), (15, true, 0), (22, true, 0)]);
        assert_eq!(got, [false, true, true, false]);
    }

    #[test]
    fn missing_hosts_wait_for_the_late_wave_even_without_progress() {
        let got = run(&[
            (0, false, 0),
            (1, true, 3),
            (22, true, 3),
            (45, true, 3),
            (52, true, 3),
        ]);
        assert_eq!(got, [false, true, true, true, false]);
    }

    #[test]
    fn routes_arriving_after_the_wait_ended_do_not_reopen_it() {
        let got = run(&[(0, false, 0), (1, true, 3), (52, true, 3), (60, true, 0)]);
        assert_eq!(got, [false, true, false, false]);
    }

    #[test]
    fn reconnecting_starts_a_fresh_wait() {
        let got = run(&[
            (0, false, 0),
            (1, true, 3),
            (100, true, 3),
            (101, false, 0),
            (102, true, 3),
        ]);
        assert_eq!(got, [false, true, false, false, true]);
    }
}

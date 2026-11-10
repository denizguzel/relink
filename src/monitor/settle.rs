//! Waiting for the VPN's routes. A VPN that connects while Re-Link watches
//! needs a while to install its routes; until then missing routes are no news.
//! Waiting lasts at least `MIN_SETTLE`, goes on while the missing set keeps
//! shrinking (no progress for `QUIET` ends it) and never exceeds `MAX_SETTLE`.
//! The same wait starts when hosts suddenly go missing while the VPN stays up
//! (FortiClient reinstalls its routes in waves without the adapter dropping).

use std::time::{Duration, Instant};

use crate::diagnose::Report;

const MIN_SETTLE: Duration = Duration::from_secs(20);
const QUIET: Duration = Duration::from_secs(20);
const MAX_SETTLE: Duration = Duration::from_secs(150);

pub(super) struct Settle {
    /// Whether the VPN was up at the previous check (None before the first).
    was_up: Option<bool>,
    /// When the VPN adapter last came up (None while down or at startup).
    up_since: Option<Instant>,
    /// Fewest missing hosts seen since then, and the most (the progress baseline).
    best: usize,
    most: usize,
    /// When the missing set last shrank.
    progress: Instant,
    /// What the previous check saw: how many hosts were missing, and whether
    /// it was waiting. A wait starts when the first goes from none to some.
    missing_before: usize,
    settling_before: bool,
}

impl Settle {
    pub(super) fn new() -> Settle {
        Settle {
            was_up: None,
            up_since: None,
            best: usize::MAX,
            most: 0,
            progress: Instant::now(),
            missing_before: 0,
            settling_before: false,
        }
    }

    /// Marks `report` as settling (and fills its progress baseline) while the
    /// routes are still arriving.
    pub(super) fn observe(&mut self, report: &mut Report) {
        self.observe_at(report, Instant::now());
    }

    /// [`Settle::observe`] at a given time, so the rules can be tested.
    fn observe_at(&mut self, report: &mut Report, now: Instant) {
        let up = report.vpn.is_some();
        if up && self.was_up == Some(false) {
            *self = Settle {
                was_up: self.was_up,
                up_since: Some(now),
                progress: now,
                ..Settle::new()
            };
        } else if !up {
            self.up_since = None;
        }
        self.was_up = Some(up);
        let missing = report.missing_hosts().len();
        // Routes vanished while the VPN stayed up: they are being reinstalled.
        if up && missing > 0 && self.missing_before == 0 && !self.settling_before {
            *self = Settle {
                was_up: self.was_up,
                up_since: Some(now),
                progress: now,
                ..Settle::new()
            };
        }
        self.missing_before = missing;
        let Some(since) = self.up_since else {
            self.settling_before = false;
            return;
        };
        self.most = self.most.max(missing);
        report.settle_total = self.most;
        if missing < self.best {
            self.best = missing;
            self.progress = now;
        }
        let elapsed = now.duration_since(since);
        report.settling = elapsed < MAX_SETTLE
            && (elapsed < MIN_SETTLE || now.duration_since(self.progress) < QUIET);
        self.settling_before = report.settling;
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::diagnose::{Adapter, Missing, Route};

    fn report(vpn: bool, missing_hosts: usize) -> Report {
        let mut r = Report::default();
        if vpn {
            r.vpn = Some(Adapter {
                index: 20,
                description: "Fortinet SSL VPN".into(),
                up: true,
                addrs: vec![Ipv4Addr::new(10, 0, 0, 2)],
            });
        }
        for i in 0..missing_hosts {
            r.missing.push(Missing {
                host: format!("h{i}.example"),
                ip: Ipv4Addr::new(1, 1, 1, i as u8),
                route: Route {
                    if_index: 7,
                    prefix_len: 0,
                    source: None,
                },
            });
        }
        r
    }

    fn at(t0: Instant, secs: u64) -> Instant {
        t0 + Duration::from_secs(secs)
    }

    /// Runs the tracker over (seconds, vpn up, missing hosts) steps and returns `settling` at each.
    fn run(steps: &[(u64, bool, usize)]) -> Vec<bool> {
        let (t0, mut s) = (Instant::now(), Settle::new());
        steps
            .iter()
            .map(|&(secs, vpn, missing)| {
                let mut r = report(vpn, missing);
                s.observe_at(&mut r, at(t0, secs));
                r.settling
            })
            .collect()
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
            (40, true, 6), // the routes went away
            (50, true, 6),
            (61, true, 6),  // 21 s without progress: a real problem
            (70, true, 0),  // all back: counts as progress, still waiting
            (95, true, 0),  // quiet for 25 s: done
            (100, true, 2), // and again later
        ]);
        assert_eq!(got, [false, false, true, true, false, true, false, true]);
    }

    #[test]
    fn waits_while_routes_keep_arriving_then_stops_when_they_stall() {
        let got = run(&[
            (0, false, 0),
            (1, true, 10), // VPN just came up
            (10, true, 7),
            (25, true, 4), // still shrinking
            (40, true, 4), // 15 s without progress: still within QUIET
            (46, true, 4), // 21 s without progress: give up waiting
        ]);
        assert_eq!(got, [false, true, true, true, true, false]);
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
    fn waits_at_least_the_minimum_even_without_progress() {
        let got = run(&[(0, false, 0), (1, true, 3), (15, true, 3), (22, true, 3)]);
        assert_eq!(got, [false, true, true, false]);
    }

    #[test]
    fn progress_baseline_is_the_most_hosts_ever_missing() {
        let (t0, mut s) = (Instant::now(), Settle::new());
        let mut totals = Vec::new();
        for (secs, vpn, missing) in [(0, false, 0), (1, true, 4), (3, true, 9), (5, true, 2)] {
            let mut r = report(vpn, missing);
            s.observe_at(&mut r, at(t0, secs));
            totals.push(r.settle_total);
        }
        assert_eq!(totals, [0, 4, 9, 9]);
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

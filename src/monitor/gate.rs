//! Asking home-network hosts whether the load balancer turns them away
//! outside the VPN (`crate::gate`), and learning the ones it does, so the
//! next analysis reports their missing routes.
//!
//! The schedule (`Schedule`) is pure; the requests run on a short-lived
//! thread so the monitor never waits for them.

use std::collections::HashMap;
use std::sync::mpsc::TryRecvError;

use super::Monitor;
use crate::config;
use crate::diagnose::Report;
use crate::gate::{self, Answer};
use crate::t;
use crate::ui::Level;

/// When a host that answered as a public site is asked again (seconds).
pub const OPEN_AGAIN: f64 = 6.0 * 3600.0;
/// When a host that did not answer is asked again.
pub const RETRY: f64 = 600.0;
/// How many hosts one round asks at most.
pub const PER_ROUND: usize = 10;

/// Which host may be asked when. Times are seconds on the monitor's clock.
#[derive(Debug, Default)]
pub struct Schedule {
    next: HashMap<String, f64>,
}

impl Schedule {
    /// The candidates due now, at most `PER_ROUND`.
    pub fn due(&self, candidates: &[String], now: f64) -> Vec<String> {
        candidates
            .iter()
            .filter(|h| self.next.get(*h).is_none_or(|&t| now >= t))
            .take(PER_ROUND)
            .cloned()
            .collect()
    }

    /// Takes a round's answers; returns the gated hosts.
    pub fn record(&mut self, answers: &[(String, Answer)], now: f64) -> Vec<String> {
        let mut gated = Vec::new();
        for (h, a) in answers {
            match a {
                Answer::Gated => {
                    // Learned from now on; asked again only once forgotten.
                    self.next.remove(h);
                    gated.push(h.clone());
                }
                Answer::Open => {
                    self.next.insert(h.clone(), now + OPEN_AGAIN);
                }
                Answer::Unknown => {
                    self.next.insert(h.clone(), now + RETRY);
                }
            }
        }
        gated
    }

    /// The VPN went down or reconnected: a new tunnel may lack other routes.
    pub fn reset(&mut self) {
        self.next.clear();
    }
}

impl Monitor {
    /// Learns the hosts a finished round found gated. Runs before the
    /// analysis, so they count as required in this very check.
    pub(super) fn collect_gate(&mut self) {
        let Some(rx) = &self.gate_rx else { return };
        let answers = match rx.try_recv() {
            Ok(a) => a,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Vec::new(),
        };
        self.gate_rx = None;
        let gated = self.gate.record(&answers, self.mono());
        if gated.is_empty() {
            return;
        }
        self.state.learn(&gated, config::now_unix());
        if let Err(e) = self.state.save(&self.paths.state) {
            self.log.line(&format!("save state: {e}"));
        }
        self.event(t!("ev_gated", hosts = gated.join(", ")), Level::Info);
    }

    /// Starts a round when hosts are due and none is in flight.
    pub(super) fn start_gate(&mut self, report: &Report) {
        if report.vpn.is_none() || report.settling {
            self.gate.reset();
            return;
        }
        if self.gate_rx.is_some() {
            return;
        }
        let required = self.state.required(&self.cfg.lock().unwrap().hosts);
        let hosts = self
            .gate
            .due(&gate::targets(report, &required), self.mono());
        if hosts.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.gate_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(gate::ask_all(&hosts));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hosts(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn new_hosts_are_due_at_once_and_a_round_is_capped() {
        let s = Schedule::default();
        let many: Vec<String> = (0..25).map(|i| format!("h{i}.example.com")).collect();
        assert_eq!(s.due(&many, 0.0).len(), PER_ROUND);
        assert_eq!(s.due(&hosts(&["a"]), 0.0), ["a"]);
    }

    #[test]
    fn public_sites_rest_long_silent_ones_briefly() {
        let mut s = Schedule::default();
        let gated = s.record(
            &[
                ("open".into(), Answer::Open),
                ("quiet".into(), Answer::Unknown),
                ("gated".into(), Answer::Gated),
            ],
            0.0,
        );
        assert_eq!(gated, ["gated"]);
        let all = hosts(&["open", "quiet", "gated"]);
        assert_eq!(s.due(&all, 1.0), ["gated"]);
        assert_eq!(s.due(&all, RETRY), ["quiet", "gated"]);
        assert_eq!(s.due(&all, OPEN_AGAIN), all);
    }

    #[test]
    fn a_new_tunnel_asks_everything_again() {
        let mut s = Schedule::default();
        s.record(&[("open".into(), Answer::Open)], 0.0);
        s.reset();
        assert_eq!(s.due(&hosts(&["open"]), 1.0), ["open"]);
    }
}

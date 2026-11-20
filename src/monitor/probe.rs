//! Whether the VPN carries traffic. FortiClient can stay "connected" after a
//! sleep or a network switch while nothing gets through its tunnel; the routes
//! look right, so only trying a connection shows it. A few company addresses
//! that route through the VPN get a short TCP connect now and then; when every
//! one times out, twice in a row, the tunnel counts as dead.
//!
//! The policy (`Probe`, `targets`) is pure; `run` does the connecting on a
//! short-lived thread so the monitor never waits for it.

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use crate::diagnose::{HostKind, Report};

/// How often a working tunnel is checked.
pub const EVERY: f64 = 60.0;
/// How soon to look again after a failed round, and while the tunnel is dead
/// (to notice at once when it works again).
pub const RETRY: f64 = 10.0;
/// Failed rounds in a row before the tunnel counts as dead.
pub const ROUNDS: u32 = 2;
/// How many addresses one round tries.
pub const TARGETS: usize = 3;
/// How long one connect may take.
pub const TIMEOUT: Duration = Duration::from_secs(4);
/// The port tried: the company hosts Re-Link watches are web sites.
pub const PORT: u16 = 443;

/// What one round found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    /// At least one address answered (even with a refusal: the packet came back).
    Alive,
    /// Every address timed out or was unreachable.
    Dead,
}

/// What changed after a round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Died,
    Recovered,
}

/// The probe state carried between checks. Times are seconds on the monitor's
/// monotonic clock.
#[derive(Debug, Default)]
pub struct Probe {
    next: f64,
    fails: u32,
    dead: bool,
}

impl Probe {
    pub fn dead(&self) -> bool {
        self.dead
    }

    /// Whether to start a round now. Only a connected, settled VPN is probed;
    /// anything else starts over (a reconnect is a new tunnel).
    pub fn due(&mut self, r: &Report, enabled: bool, now: f64) -> bool {
        if !enabled || r.vpn.is_none() || r.settling {
            *self = Probe {
                next: now + RETRY,
                ..Probe::default()
            };
            return false;
        }
        now >= self.next
    }

    /// Takes a round's result (`None`: nothing to try, which proves nothing).
    pub fn record(&mut self, round: Option<Round>, now: f64) -> Option<Change> {
        let Some(round) = round else {
            self.next = now + EVERY;
            return None;
        };
        match round {
            Round::Alive => {
                self.fails = 0;
                self.next = now + EVERY;
                std::mem::take(&mut self.dead).then_some(Change::Recovered)
            }
            Round::Dead => {
                self.fails += 1;
                self.next = now + RETRY;
                let died = !self.dead && self.fails >= ROUNDS;
                self.dead |= died;
                died.then_some(Change::Died)
            }
        }
    }
}

/// Up to `TARGETS` addresses that route through the VPN right now, one per
/// host (different hosts, so one server being down does not look like a dead
/// tunnel).
pub fn targets(r: &Report) -> Vec<Ipv4Addr> {
    let Some(vpn) = &r.vpn else { return Vec::new() };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (_, ips, kind) in &r.hosts {
        if *kind != HostKind::Vpn {
            continue;
        }
        let on_vpn = ips.iter().find(|ip| {
            r.route_of
                .get(ip)
                .copied()
                .flatten()
                .is_some_and(|rt| rt.if_index == vpn.index)
        });
        if let Some(&ip) = on_vpn
            && seen.insert(ip)
        {
            out.push(ip);
            if out.len() == TARGETS {
                break;
            }
        }
    }
    out
}

/// Whether a connect result means the address answered.
fn answered(result: std::io::Result<TcpStream>) -> bool {
    use std::io::ErrorKind::*;
    match result {
        Ok(_) => true,
        Err(e) => matches!(e.kind(), ConnectionRefused | ConnectionReset),
    }
}

/// Tries the addresses at once; `None` when there are none.
pub fn run(ips: &[Ipv4Addr]) -> Option<Round> {
    if ips.is_empty() {
        return None;
    }
    let tries: Vec<_> = ips
        .iter()
        .map(|&ip| {
            std::thread::spawn(move || {
                answered(TcpStream::connect_timeout(
                    &SocketAddr::from((ip, PORT)),
                    TIMEOUT,
                ))
            })
        })
        .collect();
    let alive = tries.into_iter().any(|t| t.join().unwrap_or(false));
    Some(if alive { Round::Alive } else { Round::Dead })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::{Adapter, Route};

    fn vpn_report(hosts: &[(&str, &[[u8; 4]], HostKind)]) -> Report {
        let mut r = Report {
            vpn: Some(Adapter {
                index: 20,
                description: "Fortinet SSL VPN".into(),
                up: true,
                addrs: vec![Ipv4Addr::new(10, 0, 0, 2)],
            }),
            ..Default::default()
        };
        for (host, ips, kind) in hosts {
            let ips: Vec<Ipv4Addr> = ips.iter().map(|&b| b.into()).collect();
            for &ip in &ips {
                let if_index = if *kind == HostKind::Vpn { 20 } else { 7 };
                r.route_of.insert(
                    ip,
                    Some(Route {
                        if_index,
                        prefix_len: 32,
                        source: None,
                    }),
                );
            }
            r.hosts.push((host.to_string(), ips, *kind));
        }
        r
    }

    #[test]
    fn one_vpn_address_per_host_at_most_three() {
        let r = vpn_report(&[
            ("a", &[[1, 1, 1, 1], [1, 1, 1, 2]], HostKind::Vpn),
            ("web", &[[8, 8, 8, 8]], HostKind::Internet),
            ("b", &[[1, 1, 1, 1]], HostKind::Vpn), // same address as a
            ("c", &[[2, 2, 2, 2]], HostKind::Vpn),
            ("d", &[[3, 3, 3, 3]], HostKind::Vpn),
            ("e", &[[4, 4, 4, 4]], HostKind::Vpn),
        ]);
        assert_eq!(
            targets(&r),
            [
                Ipv4Addr::new(1, 1, 1, 1),
                Ipv4Addr::new(2, 2, 2, 2),
                Ipv4Addr::new(3, 3, 3, 3)
            ]
        );
    }

    #[test]
    fn no_vpn_no_targets() {
        let mut r = vpn_report(&[("a", &[[1, 1, 1, 1]], HostKind::Vpn)]);
        r.vpn = None;
        assert!(targets(&r).is_empty());
    }

    #[test]
    fn two_failed_rounds_make_it_dead_and_one_answer_brings_it_back() {
        let mut p = Probe::default();
        let r = vpn_report(&[]);
        assert!(p.due(&r, true, 0.0));
        assert_eq!(p.record(Some(Round::Dead), 0.0), None, "one miss is noise");
        assert!(!p.due(&r, true, 5.0) && p.due(&r, true, RETRY));
        assert_eq!(p.record(Some(Round::Dead), RETRY), Some(Change::Died));
        assert!(p.dead());
        assert_eq!(p.record(Some(Round::Dead), 20.0), None, "said once");
        assert!(p.due(&r, true, 30.0), "a dead tunnel is retried quickly");
        assert_eq!(p.record(Some(Round::Alive), 30.0), Some(Change::Recovered));
        assert!(!p.dead() && !p.due(&r, true, 60.0) && p.due(&r, true, 90.0));
    }

    #[test]
    fn a_miss_between_answers_is_forgotten() {
        let mut p = Probe::default();
        p.record(Some(Round::Dead), 0.0);
        p.record(Some(Round::Alive), 10.0);
        assert_eq!(p.record(Some(Round::Dead), 70.0), None);
    }

    #[test]
    fn nothing_to_try_proves_nothing() {
        let mut p = Probe::default();
        assert_eq!(p.record(None, 0.0), None);
        assert!(!p.dead());
    }

    #[test]
    fn a_vpn_that_drops_or_reconnects_starts_over() {
        let mut p = Probe::default();
        let up = vpn_report(&[]);
        p.record(Some(Round::Dead), 0.0);
        p.record(Some(Round::Dead), 10.0);
        assert!(p.dead());
        assert!(!p.due(&Report::default(), true, 20.0), "VPN down");
        assert!(!p.dead());
        let mut settling = vpn_report(&[]);
        settling.settling = true;
        assert!(!p.due(&settling, true, 25.0));
        assert!(
            !p.due(&up, true, 30.0) && p.due(&up, true, 35.0),
            "a fresh start, after a moment"
        );
    }

    #[test]
    fn switched_off_it_never_runs() {
        let mut p = Probe::default();
        assert!(!p.due(&vpn_report(&[]), false, 1000.0));
    }

    #[test]
    fn a_refusal_is_an_answer_a_timeout_is_not() {
        use std::io::{Error, ErrorKind};
        assert!(answered(Err(Error::from(ErrorKind::ConnectionRefused))));
        assert!(!answered(Err(Error::from(ErrorKind::TimedOut))));
        assert!(!answered(Err(Error::from(ErrorKind::HostUnreachable))));
    }

    #[test]
    fn an_empty_round_is_no_round() {
        assert_eq!(run(&[]), None);
    }
}

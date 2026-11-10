//! Detection rules over a snapshot of the machine's network state. No OS
//! dependencies, so every rule is unit-tested; `net` collects the snapshot.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::Ipv4Addr;

/// An established TCP connection owned by a browser process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conn {
    pub pid: u32,
    pub process: String,
    pub local: Ipv4Addr,
    pub local_port: u16,
    pub remote: Ipv4Addr,
    pub remote_port: u16,
}

/// The route Windows would pick today for a destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    pub if_index: u32,
    /// 0 means the default route.
    pub prefix_len: u8,
    /// The source address Windows would use, if any.
    pub source: Option<Ipv4Addr>,
}

/// A network interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adapter {
    pub index: u32,
    pub description: String,
    pub up: bool,
    pub addrs: Vec<Ipv4Addr>,
}

/// Raw input collected from the OS.
#[derive(Default)]
pub struct Snapshot {
    pub conns: Vec<Conn>,
    pub adapters: Vec<Adapter>,
    /// Watched host -> resolved IPv4 addresses (unresolvable hosts are absent).
    pub hosts: BTreeMap<String, Vec<Ipv4Addr>>,
    /// Hosts known to belong behind the VPN (configured, or learned earlier).
    pub required: HashSet<String>,
}

/// A connection opened before a more specific (VPN) route existed, so it
/// still leaves through another interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stuck {
    pub conn: Conn,
    pub route: Route,
}

/// A watched host address that does not route through the VPN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    pub host: String,
    pub ip: Ipv4Addr,
    pub route: Route,
}

/// The overall verdict, ordered by severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Status {
    #[default]
    Ok,
    VpnDown,
    /// The VPN just connected and FortiClient is still installing its routes.
    Settling,
    Stuck,
    MissingRoute,
}

/// How one connection looks, for the Connections page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnKind {
    /// Goes through the VPN as it should.
    Vpn,
    /// Plain internet traffic over the default route.
    Internet,
    /// Bound to the wrong interface.
    Stuck,
    /// The route could not be determined.
    Unknown,
}

/// How one watched host looks, for the Hosts page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKind {
    /// Every address routes through the VPN.
    Vpn,
    /// Some or all addresses miss the VPN although they should use it.
    Missing,
    /// Never on the VPN (public site) — not a problem.
    Internet,
    /// The VPN is down, so nothing can be said.
    Unknown,
}

/// Whether a connection to `ip` can only work through the VPN, so that
/// leaving over the home network is a real fault: a private address, a host
/// route (FortiClient installs /32s for the company's FQDNs), or an address
/// of a watched host. Public addresses inside a broad range such as a /17
/// are usually CDNs that work either way.
fn vpn_only(ip: Ipv4Addr, prefix_len: u8, watched: &std::collections::HashSet<Ipv4Addr>) -> bool {
    ip.is_private() || prefix_len >= 28 || watched.contains(&ip)
}

/// Firefox keeps its sockets in the main process (`firefox.exe`, `firefox`),
/// so Re-Link cannot close them without administrator rights.
pub fn is_firefox(process: &str) -> bool {
    process.to_ascii_lowercase().starts_with("firefox")
}

/// The result of [`analyze`].
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub vpn: Option<Adapter>,
    pub stuck: Vec<Stuck>,
    pub missing: Vec<Missing>,
    /// Watched hosts with at least one address routed through the VPN right
    /// now; callers remember them as required.
    pub vpn_hosts: Vec<String>,
    /// Every browser connection with its classification.
    pub conns: Vec<(Conn, Option<Route>, ConnKind)>,
    /// Every resolved watched host with its classification.
    pub hosts: Vec<(String, Vec<Ipv4Addr>, HostKind)>,
    /// The VPN connected moments ago: missing routes are expected for now.
    pub settling: bool,
    /// Most hosts seen missing since the VPN came up: the baseline of the
    /// progress shown while its routes arrive.
    pub settle_total: usize,
    /// The route found for every address looked up (None: no route), so the
    /// Hosts page can show why a host is in its state.
    pub route_of: HashMap<Ipv4Addr, Option<Route>>,
}

impl Report {
    /// The VPN connected moments ago and its routes are still arriving, so
    /// missing routes (and the stuck connections they cause) are no news yet.
    /// An empty missing set does not end the wait: host names are still being
    /// resolved, so more addresses can show up as missing a moment later.
    pub fn waiting_for_routes(&self) -> bool {
        self.settling
    }

    /// The most severe condition in the report.
    pub fn status(&self) -> Status {
        if self.waiting_for_routes() {
            Status::Settling
        } else if !self.missing.is_empty() {
            Status::MissingRoute
        } else if !self.stuck.is_empty() {
            Status::Stuck
        } else if self.vpn.is_none() {
            Status::VpnDown
        } else {
            Status::Ok
        }
    }

    /// Distinct process names owning stuck connections, in first-seen order.
    pub fn stuck_processes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in &self.stuck {
            if !out.contains(&s.conn.process) {
                out.push(s.conn.process.clone());
            }
        }
        out
    }

    /// Distinct hosts with missing routes, in order.
    pub fn missing_hosts(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in &self.missing {
            if !out.contains(&m.host) {
                out.push(m.host.clone());
            }
        }
        out
    }

    /// A stable signature of the stuck set, so the same problem is not
    /// announced twice.
    pub fn stuck_signature(&self) -> String {
        let mut v: Vec<String> = self
            .stuck
            .iter()
            .map(|s| s.conn.remote.to_string())
            .collect();
        v.sort();
        v.dedup();
        v.join(",")
    }

    /// A stable signature of the missing set.
    pub fn missing_signature(&self) -> String {
        let mut v: Vec<String> = self
            .missing
            .iter()
            .map(|m| format!("{}={}", m.host, m.ip))
            .collect();
        v.sort();
        v.join(",")
    }
}

/// The first connected adapter whose description contains one of the
/// keywords (case-insensitive) and that has an IPv4 address.
pub fn find_vpn<'a>(adapters: &'a [Adapter], keywords: &[String]) -> Option<&'a Adapter> {
    adapters.iter().find(|a| {
        let desc = a.description.to_lowercase();
        a.up && !a.addrs.is_empty()
            && keywords
                .iter()
                .any(|k| !k.is_empty() && desc.contains(&k.to_lowercase()))
    })
}

/// Applies the detection rules.
///
/// A connection is stuck when Windows now has a specific (non-default) route
/// to its remote address whose source differs from the address the socket is
/// bound to. That is what happens when FortiClient adds a /32 route for a
/// hostname after the browser already connected over the home network: the
/// socket keeps using the old interface and the server sees the wrong client
/// IP. Failed route lookups are skipped, not reported.
///
/// Watched hosts only matter while the VPN is up. A host counts as a VPN host
/// when one of its addresses routes through the VPN now, or it is required. A
/// VPN host misses its route when any address does not go through the VPN:
/// either the split tunnel covers only some of the load balancer's IPs, or
/// all of them changed since FortiClient installed routes. Hosts that never
/// routed through the VPN (public sites) are never reported.
pub fn analyze(
    s: &Snapshot,
    vpn_keywords: &[String],
    mut route: impl FnMut(Ipv4Addr) -> Option<Route>,
) -> Report {
    let mut r = Report {
        vpn: find_vpn(&s.adapters, vpn_keywords).cloned(),
        ..Default::default()
    };
    let mut cache: HashMap<Ipv4Addr, Option<Route>> = HashMap::new();
    let mut lookup = |ip: Ipv4Addr| *cache.entry(ip).or_insert_with(|| route(ip));

    let watched_ips: std::collections::HashSet<Ipv4Addr> =
        s.hosts.values().flatten().copied().collect();
    for c in &s.conns {
        if c.remote.is_loopback() || c.remote.is_unspecified() {
            continue;
        }
        let rt = lookup(c.remote);
        let kind = match rt {
            None => ConnKind::Unknown,
            Some(rt) if rt.prefix_len > 0 && rt.source.is_some_and(|src| src != c.local) => {
                if vpn_only(c.remote, rt.prefix_len, &watched_ips) {
                    r.stuck.push(Stuck {
                        conn: c.clone(),
                        route: rt,
                    });
                    ConnKind::Stuck
                } else {
                    // A public address that merely falls inside a broad VPN
                    // range (a CDN, say): it answers over the home network too.
                    ConnKind::Internet
                }
            }
            Some(rt) if r.vpn.as_ref().is_some_and(|v| v.index == rt.if_index) => ConnKind::Vpn,
            Some(_) => ConnKind::Internet,
        };
        r.conns.push((c.clone(), rt, kind));
    }

    for (host, ips) in &s.hosts {
        let Some(vpn) = &r.vpn else {
            r.hosts.push((host.clone(), ips.clone(), HostKind::Unknown));
            continue;
        };
        let mut off = Vec::new();
        let mut on_vpn = false;
        for &ip in ips {
            match lookup(ip) {
                None => {}
                Some(rt) if rt.if_index == vpn.index => on_vpn = true,
                Some(rt) => off.push(Missing {
                    host: host.clone(),
                    ip,
                    route: rt,
                }),
            }
        }
        if on_vpn {
            r.vpn_hosts.push(host.clone());
        }
        let kind = if (on_vpn || s.required.contains(host)) && !off.is_empty() {
            r.missing.extend(off);
            HostKind::Missing
        } else if on_vpn {
            HostKind::Vpn
        } else {
            HostKind::Internet
        };
        r.hosts.push((host.clone(), ips.clone(), kind));
    }
    r.route_of = cache;
    r
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_empty_missing_set_does_not_end_the_wait_after_the_vpn_connects() {
        let mut r = Report {
            vpn: Some(Adapter {
                index: 20,
                description: "Fortinet SSL VPN".into(),
                up: true,
                addrs: vec![],
            }),
            settling: true,
            ..Report::default()
        };
        assert_eq!(r.status(), Status::Settling, "not healthy yet");
        r.settling = false;
        assert_eq!(r.status(), Status::Ok);
    }

    use super::*;

    const HOME: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 108);
    const VPN_IP: Ipv4Addr = Ipv4Addr::new(10, 34, 33, 26);
    const ALB_NEW: Ipv4Addr = Ipv4Addr::new(52, 50, 137, 113);
    const ALB_OLD: Ipv4Addr = Ipv4Addr::new(34, 253, 124, 119);
    const PUBLIC: Ipv4Addr = Ipv4Addr::new(142, 250, 187, 110);
    const LAN: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 1);
    const FAILS: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 9);
    const HOME_IF: u32 = 7;
    const VPN_IF: u32 = 23;

    fn adapters(vpn_up: bool) -> Vec<Adapter> {
        vec![
            Adapter {
                index: HOME_IF,
                description: "Intel(R) Ethernet Controller (3) I225-V".into(),
                up: true,
                addrs: vec![HOME],
            },
            Adapter {
                index: 30,
                description: "Fortinet Virtual Ethernet Adapter (NDIS 6.30)".into(),
                up: false,
                addrs: vec![],
            },
            Adapter {
                index: VPN_IF,
                description: "Fortinet SSL VPN Virtual Ethernet Adapter".into(),
                up: vpn_up,
                addrs: vec![VPN_IP],
            },
        ]
    }

    /// A split-tunnel table: ALB_NEW has a /32 via the VPN, the LAN is
    /// on-link, everything else uses the home default route.
    fn routes(dst: Ipv4Addr) -> Option<Route> {
        if dst == ALB_NEW {
            Some(Route {
                if_index: VPN_IF,
                prefix_len: 32,
                source: Some(VPN_IP),
            })
        } else if dst.octets()[..3] == [192, 168, 1] {
            Some(Route {
                if_index: HOME_IF,
                prefix_len: 24,
                source: Some(HOME),
            })
        } else if dst == FAILS {
            None
        } else {
            Some(Route {
                if_index: HOME_IF,
                prefix_len: 0,
                source: Some(HOME),
            })
        }
    }

    fn kw() -> Vec<String> {
        vec!["Fortinet SSL VPN".into()]
    }

    fn conn(pid: u32, process: &str, local: Ipv4Addr, remote: Ipv4Addr, port: u16) -> Conn {
        Conn {
            pid,
            process: process.into(),
            local,
            local_port: 50000,
            remote,
            remote_port: port,
        }
    }

    #[test]
    fn stuck_connection_on_home_interface() {
        let s = Snapshot {
            conns: vec![
                conn(100, "chrome.exe", HOME, ALB_NEW, 443),   // stuck
                conn(100, "chrome.exe", VPN_IP, ALB_NEW, 443), // fine, via VPN
                conn(100, "chrome.exe", HOME, PUBLIC, 443),    // default route
                conn(100, "chrome.exe", HOME, LAN, 80),        // on-link
                conn(200, "msedge.exe", HOME, FAILS, 443),     // lookup fails
            ],
            adapters: adapters(true),
            ..Default::default()
        };
        let r = analyze(&s, &kw(), routes);
        assert_eq!(r.stuck.len(), 1);
        assert_eq!(
            (r.stuck[0].conn.local, r.stuck[0].conn.remote),
            (HOME, ALB_NEW)
        );
        assert_eq!(r.stuck_processes(), vec!["chrome.exe".to_string()]);
        assert_eq!(r.status(), Status::Stuck);
        let kinds: Vec<ConnKind> = r.conns.iter().map(|c| c.2).collect();
        assert_eq!(
            kinds,
            vec![
                ConnKind::Stuck,
                ConnKind::Vpn,
                ConnKind::Internet,
                ConnKind::Internet,
                ConnKind::Unknown
            ]
        );
    }

    #[test]
    fn missing_route_rules() {
        let hosts = BTreeMap::from([
            ("partial.example".to_string(), vec![ALB_NEW, ALB_OLD]), // the 1-in-3 403s
            ("moved.example".to_string(), vec![ALB_OLD]), // all IPs changed; known VPN host
            ("public.example".to_string(), vec![PUBLIC]), // never on the VPN
            ("good.example".to_string(), vec![ALB_NEW]),
        ]);
        let required = HashSet::from(["moved.example".to_string()]);
        let s = Snapshot {
            adapters: adapters(true),
            hosts: hosts.clone(),
            required: required.clone(),
            ..Default::default()
        };
        let r = analyze(&s, &kw(), routes);
        let got: Vec<String> = r
            .missing
            .iter()
            .map(|m| format!("{}={}", m.host, m.ip))
            .collect();
        assert_eq!(
            got,
            vec![
                format!("moved.example={ALB_OLD}"),
                format!("partial.example={ALB_OLD}")
            ]
        );
        assert_eq!(
            r.vpn_hosts,
            vec!["good.example".to_string(), "partial.example".to_string()]
        );
        assert_eq!(r.status(), Status::MissingRoute);
        let kinds: HashMap<&str, HostKind> = r.hosts.iter().map(|h| (h.0.as_str(), h.2)).collect();
        assert_eq!(kinds["public.example"], HostKind::Internet);
        assert_eq!(kinds["good.example"], HostKind::Vpn);
        assert_eq!(kinds["moved.example"], HostKind::Missing);

        let s = Snapshot {
            adapters: adapters(false),
            hosts,
            required,
            ..Default::default()
        };
        let r = analyze(&s, &kw(), routes);
        assert!(r.missing.is_empty() && r.vpn.is_none());
        assert_eq!(r.status(), Status::VpnDown);
        assert!(r.hosts.iter().all(|h| h.2 == HostKind::Unknown));
    }

    #[test]
    fn all_good() {
        let s = Snapshot {
            conns: vec![conn(1, "chrome.exe", VPN_IP, ALB_NEW, 443)],
            adapters: adapters(true),
            hosts: BTreeMap::from([("a".to_string(), vec![ALB_NEW])]),
            ..Default::default()
        };
        let r = analyze(&s, &kw(), routes);
        assert_eq!(r.status(), Status::Ok);
        assert_eq!(r.vpn.as_ref().map(|v| v.index), Some(VPN_IF));
    }

    #[test]
    fn find_vpn_ignores_disconnected_and_case() {
        assert!(find_vpn(&adapters(false), &["fortinet".into()]).is_none());
        assert_eq!(
            find_vpn(&adapters(true), &["FORTINET SSL".into()]).map(|a| a.index),
            Some(VPN_IF)
        );
    }

    #[test]
    fn signatures_are_order_independent() {
        let mk = |ips: &[Ipv4Addr]| Report {
            stuck: ips
                .iter()
                .map(|&ip| Stuck {
                    conn: conn(1, "c", HOME, ip, 443),
                    route: routes(ALB_NEW).unwrap(),
                })
                .collect(),
            ..Default::default()
        };
        assert_eq!(
            mk(&[ALB_NEW, ALB_OLD, ALB_NEW]).stuck_signature(),
            mk(&[ALB_OLD, ALB_NEW]).stuck_signature()
        );
    }

    #[test]
    fn broad_vpn_range_over_a_public_address_is_not_stuck() {
        let cdn = Ipv4Addr::new(65, 9, 9, 92);
        let route = |dst: Ipv4Addr| {
            if dst == cdn {
                Some(Route {
                    if_index: VPN_IF,
                    prefix_len: 17,
                    source: Some(VPN_IP),
                })
            } else {
                routes(dst)
            }
        };
        let s = Snapshot {
            conns: vec![conn(1, "chrome.exe", HOME, cdn, 443)],
            adapters: adapters(true),
            ..Default::default()
        };
        let r = analyze(&s, &kw(), route);
        assert!(r.stuck.is_empty());
        assert_eq!(r.conns[0].2, ConnKind::Internet);
        // The same address becomes stuck once a watched host resolves to it.
        let mut s = s;
        s.hosts.insert("known.example".into(), vec![cdn]);
        assert_eq!(analyze(&s, &kw(), route).stuck.len(), 1);
    }
}

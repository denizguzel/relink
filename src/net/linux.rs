//! Linux network state: TCP sockets from `/proc/net/tcp`, their owners from
//! `/proc/<pid>/fd`, adapters and routes from iproute2's JSON output
//! (`ip -j`). Reading needs no privileges for the user's own processes.

use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::{Process, TcpConn};
use crate::diagnose::{Adapter, Route};

// ---------- parsers (pure, unit-tested) ----------

/// One row of `/proc/net/tcp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcTcp {
    pub local: Ipv4Addr,
    pub local_port: u16,
    pub remote: Ipv4Addr,
    pub remote_port: u16,
    pub inode: u64,
}

fn hex_addr(s: &str) -> Option<(Ipv4Addr, u16)> {
    let (ip, port) = s.split_once(':')?;
    // The address is the raw in_addr printed as a host-order u32.
    let ip = Ipv4Addr::from(u32::from_str_radix(ip, 16).ok()?.to_le_bytes());
    Some((ip, u16::from_str_radix(port, 16).ok()?))
}

/// Established IPv4 sockets in `/proc/net/tcp` text.
pub fn parse_proc_net_tcp(text: &str) -> Vec<ProcTcp> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() < 10 || f[3] != "01" {
                return None; // 01 = TCP_ESTABLISHED
            }
            let (local, local_port) = hex_addr(f[1])?;
            let (remote, remote_port) = hex_addr(f[2])?;
            Some(ProcTcp {
                local,
                local_port,
                remote,
                remote_port,
                inode: f[9].parse().ok()?,
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct IpLink {
    ifindex: u32,
    ifname: String,
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    operstate: String,
    #[serde(default)]
    addr_info: Vec<IpAddrInfo>,
}

#[derive(Deserialize)]
struct IpAddrInfo {
    #[serde(default)]
    family: String,
    #[serde(default)]
    local: String,
}

/// Adapters from `ip -j -4 addr show`. The description is the interface
/// name; tunnels report operstate UNKNOWN, so "up" means the UP flag and not
/// DOWN.
pub fn parse_ip_addr(json: &str) -> Result<Vec<Adapter>, String> {
    let links: Vec<IpLink> = serde_json::from_str(json).map_err(|e| format!("ip addr: {e}"))?;
    Ok(links
        .into_iter()
        .filter(|l| l.ifname != "lo")
        .map(|l| Adapter {
            index: l.ifindex,
            up: l.flags.iter().any(|f| f == "UP") && l.operstate != "DOWN",
            addrs: l
                .addr_info
                .iter()
                .filter(|a| a.family == "inet")
                .filter_map(|a| a.local.parse().ok())
                .collect(),
            description: l.ifname,
        })
        .collect())
}

#[derive(Deserialize, Clone, Debug)]
pub struct IpRoute {
    #[serde(default)]
    pub dst: String,
    #[serde(default)]
    pub dev: String,
    #[serde(default)]
    pub prefsrc: String,
    #[serde(default)]
    pub table: String,
}

pub fn parse_routes(json: &str) -> Result<Vec<IpRoute>, String> {
    serde_json::from_str(json).map_err(|e| format!("ip route: {e}"))
}

/// The prefix length of the most specific route on `dev` that contains
/// `ip` (0 for the default route); `None` when no listed route matches.
pub fn matching_prefix(routes: &[IpRoute], dev: &str, ip: Ipv4Addr) -> Option<u8> {
    routes
        .iter()
        .filter(|r| r.dev == dev)
        .filter_map(|r| {
            if r.dst == "default" {
                return Some(0);
            }
            let (net, len) = match r.dst.split_once('/') {
                Some((n, l)) => (n.parse::<Ipv4Addr>().ok()?, l.parse::<u8>().ok()?),
                None => (r.dst.parse::<Ipv4Addr>().ok()?, 32),
            };
            let mask = if len == 0 {
                0
            } else {
                u32::MAX << (32 - len.min(32))
            };
            (u32::from(ip) & mask == u32::from(net) & mask).then_some(len)
        })
        .max()
}

// ---------- OS access ----------

fn ip_json(args: &[&str]) -> Result<String, String> {
    let out = Command::new("ip")
        .arg("-j")
        .args(args)
        .output()
        .map_err(|e| format!("ip: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ip {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Adapters and the route list, cached for a moment so the many
/// `best_route` calls of one check share them.
struct Tables {
    at: Instant,
    adapters: Vec<Adapter>,
    routes: Vec<IpRoute>,
}
static TABLES: Mutex<Option<Tables>> = Mutex::new(None);

fn tables() -> Result<(Vec<Adapter>, Vec<IpRoute>), String> {
    let mut t = TABLES.lock().unwrap();
    if let Some(t) = t
        .as_ref()
        .filter(|t| t.at.elapsed() < Duration::from_secs(2))
    {
        return Ok((t.adapters.clone(), t.routes.clone()));
    }
    let adapters = parse_ip_addr(&ip_json(&["-4", "addr", "show"])?)?;
    let routes = parse_routes(&ip_json(&["-4", "route", "show", "table", "all"])?)?;
    *t = Some(Tables {
        at: Instant::now(),
        adapters: adapters.clone(),
        routes: routes.clone(),
    });
    Ok((adapters, routes))
}

pub fn adapters() -> Result<Vec<Adapter>, String> {
    Ok(tables()?.0)
}

/// The kernel's choice for `dst` (`ip route get`), with the prefix length of
/// the route it came from.
pub fn best_route(dst: Ipv4Addr) -> Option<Route> {
    let got = parse_routes(&ip_json(&["-4", "route", "get", &dst.to_string()]).ok()?).ok()?;
    let r = got.first()?;
    let (adapters, routes) = tables().ok()?;
    let if_index = adapters.iter().find(|a| a.description == r.dev)?.index;
    let prefix_len = matching_prefix(&routes, &r.dev, dst).unwrap_or(32);
    Some(Route {
        if_index,
        prefix_len,
        source: r.prefsrc.parse().ok(),
    })
}

fn exe_name(pid: u32) -> Option<String> {
    let exe = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    Some(
        exe.file_name()?
            .to_string_lossy()
            .trim_end_matches(" (deleted)")
            .to_string(),
    )
}

fn pids() -> impl Iterator<Item = u32> {
    fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse().ok())
}

/// The user's processes whose executable name matches one of `names`
/// (case-insensitive, ".exe" ignored); all readable ones when empty.
pub fn processes(names: &[String]) -> Result<Vec<Process>, String> {
    let wanted: Vec<String> = names
        .iter()
        .map(|n| n.trim_end_matches(".exe").to_lowercase())
        .collect();
    Ok(pids()
        .filter_map(|pid| {
            let name = exe_name(pid)?;
            (wanted.is_empty() || wanted.contains(&name.to_lowercase()))
                .then_some(Process { pid, name })
        })
        .collect())
}

pub fn command_line(pid: u32) -> Option<String> {
    let raw = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    Some(
        raw.split(|&b| b == 0)
            .filter(|a| !a.is_empty())
            .map(|a| String::from_utf8_lossy(a))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub fn terminate(pid: u32) -> Result<(), String> {
    // SAFETY: kill(2) with a pid and a signal number has no memory effects.
    if unsafe { libc::kill(pid as i32, libc::SIGKILL) } == 0 {
        Ok(())
    } else {
        Err(format!("kill({pid}): {}", std::io::Error::last_os_error()))
    }
}

/// Established IPv4 TCP connections with owner PIDs (the user's own
/// processes; sockets of others keep pid 0).
pub fn established_tcp() -> Result<Vec<TcpConn>, String> {
    let rows = parse_proc_net_tcp(
        &fs::read_to_string("/proc/net/tcp").map_err(|e| format!("/proc/net/tcp: {e}"))?,
    );
    let mut owner: HashMap<u64, u32> = HashMap::new();
    for pid in pids() {
        let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if let Ok(target) = fs::read_link(fd.path())
                && let Some(inode) = target
                    .to_str()
                    .and_then(|t| t.strip_prefix("socket:["))
                    .and_then(|t| t.strip_suffix(']'))
                && let Ok(inode) = inode.parse()
            {
                owner.insert(inode, pid);
            }
        }
    }
    Ok(rows
        .into_iter()
        .map(|r| TcpConn {
            pid: owner.get(&r.inode).copied().unwrap_or(0),
            local: r.local,
            local_port: r.local_port,
            remote: r.remote,
            remote_port: r.remote_port,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_established_rows_only() {
        let text = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 3500007F:0035 00000000:0000 0A 00000000:00000000 00:00000000 00000000   101        0 1234 1 0 100 0 0 10 0\n\
   1: 6C01A8C0:D5E9 71893234:01BB 01 00000000:00000000 02:000A0D5C 00000000  1000        0 98765 2 0 20 4 30 10 -1\n";
        let rows = parse_proc_net_tcp(text);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].local, Ipv4Addr::new(192, 168, 1, 108));
        assert_eq!(rows[0].local_port, 0xD5E9);
        assert_eq!(rows[0].remote, Ipv4Addr::new(52, 50, 137, 113));
        assert_eq!(rows[0].remote_port, 443);
        assert_eq!(rows[0].inode, 98765);
    }

    #[test]
    fn parses_ip_addr_json_and_treats_tunnels_as_up() {
        let json = r#"[
          {"ifindex":1,"ifname":"lo","flags":["LOOPBACK","UP"],"operstate":"UNKNOWN","addr_info":[{"family":"inet","local":"127.0.0.1"}]},
          {"ifindex":2,"ifname":"wlp3s0","flags":["BROADCAST","MULTICAST","UP","LOWER_UP"],"operstate":"UP","addr_info":[{"family":"inet","local":"192.168.1.108"}]},
          {"ifindex":9,"ifname":"vpn","flags":["POINTOPOINT","NOARP","UP","LOWER_UP"],"operstate":"UNKNOWN","addr_info":[{"family":"inet","local":"10.34.33.26"}]},
          {"ifindex":4,"ifname":"docker0","flags":["NO-CARRIER","UP"],"operstate":"DOWN","addr_info":[{"family":"inet","local":"172.17.0.1"}]}
        ]"#;
        let a = parse_ip_addr(json).unwrap();
        assert_eq!(
            a.iter()
                .map(|a| (a.description.as_str(), a.up))
                .collect::<Vec<_>>(),
            vec![("wlp3s0", true), ("vpn", true), ("docker0", false)]
        );
        assert_eq!(a[1].addrs, vec![Ipv4Addr::new(10, 34, 33, 26)]);
    }

    #[test]
    fn finds_the_most_specific_matching_route() {
        let routes = parse_routes(
            r#"[{"dst":"default","dev":"wlp3s0"},{"dst":"192.168.1.0/24","dev":"wlp3s0"},
                {"dst":"52.50.137.113","dev":"vpn"},{"dst":"52.48.0.0/14","dev":"vpn"},{"dst":"10.0.0.0/8","dev":"vpn","table":"fct"}]"#,
        )
        .unwrap();
        let ip = |s: &str| s.parse::<Ipv4Addr>().unwrap();
        assert_eq!(
            matching_prefix(&routes, "vpn", ip("52.50.137.113")),
            Some(32)
        );
        assert_eq!(matching_prefix(&routes, "vpn", ip("52.49.1.1")), Some(14));
        assert_eq!(matching_prefix(&routes, "wlp3s0", ip("8.8.8.8")), Some(0));
        assert_eq!(
            matching_prefix(&routes, "wlp3s0", ip("192.168.1.1")),
            Some(24)
        );
        assert_eq!(matching_prefix(&routes, "vpn", ip("8.8.8.8")), None);
    }
}

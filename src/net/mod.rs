//! Collects a [`Snapshot`] from the OS and restarts browser network services.
//! The Windows implementation lives in `windows.rs`; elsewhere every call
//! reports that it is unsupported, so the UI still builds and renders.

use std::collections::{BTreeMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::diagnose::{Conn, Snapshot};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{adapters, best_route, command_line, established_tcp, processes, terminate};
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::{adapters, best_route, command_line, established_tcp, processes, terminate};

/// One row of the IPv4 TCP table.
#[derive(Clone, Debug)]
pub struct TcpConn {
    pub pid: u32,
    pub local: Ipv4Addr,
    pub local_port: u16,
    pub remote: Ipv4Addr,
    pub remote_port: u16,
}

/// A running process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub name: String,
}

#[cfg(not(any(windows, target_os = "linux")))]
mod unsupported {
    use super::*;
    use crate::diagnose::Route;
    const MSG: &str = "Re-Link reads the network state on Windows only";
    pub fn best_route(_: Ipv4Addr) -> Option<Route> {
        None
    }
    pub fn established_tcp() -> Result<Vec<TcpConn>, String> {
        Err(MSG.into())
    }
    pub fn adapters() -> Result<Vec<crate::diagnose::Adapter>, String> {
        Err(MSG.into())
    }
    pub fn processes(_: &[String]) -> Result<Vec<Process>, String> {
        Err(MSG.into())
    }
    pub fn command_line(_: u32) -> Option<String> {
        None
    }
    pub fn terminate(_: u32) -> Result<(), String> {
        Err(MSG.into())
    }
}
#[cfg(not(any(windows, target_os = "linux")))]
pub use unsupported::*;

/// Marks Chromium's network-service utility process (Chrome, Edge, Brave…).
pub const NETWORK_SERVICE_FLAG: &str = "--utility-sub-type=network.mojom.NetworkService";

/// The processes to watch: the configured ones plus the built-in browsers
/// (so a settings file written by an older version still covers browsers
/// added since, like Firefox). An empty
/// list still means "every process".
pub fn watched(browsers: &[String]) -> Vec<String> {
    let mut v = browsers.to_vec();
    if v.is_empty() {
        return v;
    }
    for name in crate::config::default_browsers() {
        if !v.iter().any(|b| b.eq_ignore_ascii_case(&name)) {
            v.push(name);
        }
    }
    v
}

/// Whether a process command line is a browser's network process: Chromium's
/// network-service utility, or Firefox's socket process (a content process
/// whose type, the last argument, is `socket`). Firefox starts a new socket
/// process on its own, and falls back to networking in the main process if
/// it cannot.
pub fn is_network_process(command_line: &str) -> bool {
    command_line.contains(NETWORK_SERVICE_FLAG)
        || (command_line.contains("-contentproc") && command_line.trim_end().ends_with(" socket"))
}

/// The given browsers' network-service processes.
pub fn network_services(browsers: &[String]) -> Result<Vec<Process>, String> {
    Ok(processes(browsers)?
        .into_iter()
        .filter(|p| command_line(p.pid).is_some_and(|cl| is_network_process(&cl)))
        .collect())
}

/// Kills the browsers' network-service processes. Chromium starts a fresh
/// one within a second or two, so tabs survive while every pooled socket and
/// the in-browser DNS cache are dropped. Returns the names of the ended
/// processes and the errors for the rest.
pub fn restart_network_services(browsers: &[String]) -> (Vec<String>, Vec<String>) {
    let procs = match network_services(browsers) {
        Ok(p) => p,
        Err(e) => return (Vec::new(), vec![e]),
    };
    let mut names = Vec::new();
    let mut errs = Vec::new();
    for p in procs {
        match terminate(p.pid) {
            Ok(()) => names.push(p.name.clone()),
            Err(e) => errs.push(format!("{} ({}): {e}", p.name, p.pid)),
        }
    }
    (names, errs)
}

/// Hosts with a lookup still running, so a slow DNS never piles up threads.
static IN_FLIGHT: LazyLock<Arc<Mutex<HashSet<String>>>> = LazyLock::new(Default::default);

/// Resolves the IPv4 addresses of `hosts` in parallel (8 at a time) through
/// the system resolver, waiting at most `timeout`. Hosts that fail or are
/// still resolving are left out.
pub fn resolve(hosts: &[String], timeout: Duration) -> BTreeMap<String, Vec<Ipv4Addr>> {
    let (tx, rx) = mpsc::channel::<(String, Vec<Ipv4Addr>)>();
    let queue = Arc::new(Mutex::new(
        hosts
            .iter()
            .filter(|h| IN_FLIGHT.lock().unwrap().insert((*h).clone()))
            .cloned()
            .collect::<Vec<_>>(),
    ));
    let pending = queue.lock().unwrap().len();
    for _ in 0..pending.min(8) {
        let (queue, tx) = (queue.clone(), tx.clone());
        std::thread::spawn(move || {
            while let Some(h) = queue.lock().unwrap().pop() {
                let ips: Vec<Ipv4Addr> = (h.as_str(), 443)
                    .to_socket_addrs()
                    .map(|it| {
                        it.filter_map(|a| match a {
                            SocketAddr::V4(v4) => Some(*v4.ip()),
                            SocketAddr::V6(_) => None,
                        })
                        .collect()
                    })
                    .unwrap_or_default();
                IN_FLIGHT.lock().unwrap().remove(&h);
                if tx.send((h, ips)).is_err() {
                    // The caller gave up; keep draining so the in-flight set empties.
                }
            }
        });
    }
    drop(tx);
    let deadline = Instant::now() + timeout;
    let mut out = BTreeMap::new();
    for _ in 0..pending {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok((h, mut ips)) if !ips.is_empty() => {
                ips.sort();
                ips.dedup();
                out.insert(h, ips);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    out
}

/// Browser-owned connections, adapters and the watched hosts' addresses.
pub fn collect(browsers: &[String], hosts: &[String]) -> Result<Snapshot, String> {
    let names: std::collections::HashMap<u32, String> = processes(&watched(browsers))?
        .into_iter()
        .map(|p| (p.pid, p.name))
        .collect();
    let conns = established_tcp()?
        .into_iter()
        .filter_map(|c| {
            names.get(&c.pid).map(|name| Conn {
                pid: c.pid,
                process: name.clone(),
                local: c.local,
                local_port: c.local_port,
                remote: c.remote,
                remote_port: c.remote_port,
            })
        })
        .collect();
    Ok(Snapshot {
        conns,
        adapters: adapters()?,
        hosts: resolve(hosts, Duration::from_secs(5)),
        required: HashSet::new(),
    })
}

#[cfg(test)]
mod network_process_tests {
    use super::is_network_process;

    #[test]
    fn recognizes_chromium_and_firefox_network_processes() {
        assert!(is_network_process(
            "chrome.exe --type=utility --utility-sub-type=network.mojom.NetworkService --lang=en"
        ));
        assert!(is_network_process(
            r#"firefox.exe -contentproc --channel="123" -appDir "C:\Firefox" - 42 7 socket"#
        ));
        assert!(!is_network_process(
            r#"firefox.exe -contentproc --channel="123" - 42 7 tab"#
        ));
        assert!(!is_network_process("firefox.exe -no-remote"));
        assert!(!is_network_process("chrome.exe --type=renderer"));
    }
}

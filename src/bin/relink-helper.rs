//! relink-helper: the only part of Re-Link that runs as root (Linux). The
//! app calls it through `sudo -n` after `scripts/install-linux.sh` put it in
//! /usr/local/bin and allowed it in /etc/sudoers.d/relink. It does exactly
//! three things and validates every argument:
//!
//! ```text
//! relink-helper probe                       prints "ok"
//! relink-helper route-add <ipv4>            /32 route to <ipv4> through the VPN tunnel
//! relink-helper route-del <ipv4>            removes a route this helper added
//! relink-helper kill-socket <lip> <lport> <rip> <rport>
//!                                           closes one TCP socket (ss -K)
//! ```
//!
//! The VPN interface is found here (an UP interface named like FortiClient's
//! tunnel or ppp, holding an IPv4 address), never taken from the caller.
//! Routes it adds carry `proto 222`, so it only ever deletes its own.

use std::net::Ipv4Addr;
use std::process::{Command, ExitCode};

const PROTO: &str = "222";
const VPN_NAME_HINTS: &[&str] = &["vpn", "fct", "forti", "ppp"];

fn main() -> ExitCode {
    if !cfg!(target_os = "linux") {
        eprintln!("relink-helper is for Linux only");
        return ExitCode::from(2);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["probe"] => Ok("ok".to_string()),
        ["route-add", ip] => ipv4(ip).and_then(route_add),
        ["route-del", ip] => ipv4(ip).and_then(route_del),
        ["kill-socket", lip, lport, rip, rport] => (|| kill_socket(ipv4(lip)?, port(lport)?, ipv4(rip)?, port(rport)?))(),
        _ => Err("usage: relink-helper probe | route-add <ipv4> | route-del <ipv4> | kill-socket <lip> <lport> <rip> <rport>".into()),
    };
    match result {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("relink-helper: {e}");
            ExitCode::from(2)
        }
    }
}

fn ipv4(s: &str) -> Result<Ipv4Addr, String> {
    let ip: Ipv4Addr = s.parse().map_err(|_| format!("not an IPv4 address: {s}"))?;
    if ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_link_local()
    {
        return Err(format!("refusing special address {ip}"));
    }
    Ok(ip)
}

fn port(s: &str) -> Result<u16, String> {
    s.parse::<u16>()
        .ok()
        .filter(|p| *p != 0)
        .ok_or(format!("not a port: {s}"))
}

/// The first of the given absolute paths that exists (sudo's PATH is not
/// trusted).
fn tool(candidates: &[&'static str]) -> Result<&'static str, String> {
    candidates
        .iter()
        .copied()
        .find(|p| std::path::Path::new(p).exists())
        .ok_or(format!("{} not found", candidates[0]))
}

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .env_clear()
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn ip_cmd() -> Result<&'static str, String> {
    tool(&["/usr/sbin/ip", "/sbin/ip", "/usr/bin/ip", "/bin/ip"])
}

/// The VPN tunnel interface's name.
fn vpn_interface() -> Result<String, String> {
    let json = run(ip_cmd()?, &["-j", "-4", "addr", "show", "up"])?;
    let links: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("ip addr: {e}"))?;
    links
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["addr_info"].as_array().is_some_and(|a| !a.is_empty()))
        .filter_map(|l| l["ifname"].as_str())
        .find(|name| {
            let n = name.to_ascii_lowercase();
            VPN_NAME_HINTS.iter().any(|h| n.contains(h))
        })
        .map(str::to_owned)
        .ok_or("no VPN interface is up".into())
}

fn route_add(ip: Ipv4Addr) -> Result<String, String> {
    let dev = vpn_interface()?;
    let dst = format!("{ip}/32");
    run(
        ip_cmd()?,
        &["-4", "route", "replace", &dst, "dev", &dev, "proto", PROTO],
    )?;
    Ok(format!("route {dst} dev {dev}"))
}

fn route_del(ip: Ipv4Addr) -> Result<String, String> {
    let dst = format!("{ip}/32");
    run(ip_cmd()?, &["-4", "route", "del", &dst, "proto", PROTO])?;
    Ok(format!("deleted {dst}"))
}

fn kill_socket(lip: Ipv4Addr, lport: u16, rip: Ipv4Addr, rport: u16) -> Result<String, String> {
    let ss = tool(&["/usr/bin/ss", "/usr/sbin/ss", "/bin/ss", "/sbin/ss"])?;
    let (src, dst) = (format!("{lip}:{lport}"), format!("{rip}:{rport}"));
    run(ss, &["-K", "-t", "-n", "src", &src, "dst", &dst])?;
    Ok(format!("closed {src} -> {dst}"))
}

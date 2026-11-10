//! The command-line modes: help, a read-only check, the bug-report dump and a
//! manual fix.

use std::process::ExitCode;

use relink::config::{self, Config, Paths, State};
use relink::diagnose::{self, Report, Status};
use relink::{VERSION, bookmarks, net, ui};

pub fn help() {
    println!(
        "Re-Link {VERSION}\n\n  relink               tray app and window\n  relink --hidden      start in the tray\n  relink --check [-v]  print a read-only diagnosis (exit 3 on a problem)\n  relink --dump        --check -v plus the raw network state, for bug reports\n  relink --fix         restart the browsers' network service\n  relink --version"
    );
}

/// Restarts the browsers' network services, like "Fix now".
pub fn fix(cfg: &Config) -> ExitCode {
    let (names, errs) = net::restart_network_services(&net::watched(&cfg.browsers));
    println!(
        "restarted {} network service process(es): {}",
        names.len(),
        names.join(", ")
    );
    for e in &errs {
        eprintln!("{e}");
    }
    if errs.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Prints the diagnosis; the exit code is 3 when something is wrong.
pub fn check(paths: &Paths, cfg: &Config, cfg_err: Option<String>, verbose: bool) -> ExitCode {
    if let Some(e) = cfg_err {
        eprintln!("config: {e}");
    }
    let state = State::load(&paths.state);
    let mut hosts = cfg.hosts.clone();
    if cfg.use_bookmarks {
        let files = bookmarks::all_files();
        hosts = config::merge(&[
            &hosts,
            &bookmarks::from_files(&files, &cfg.bookmark_domains),
        ]);
    }
    let mut snap = match net::collect(&cfg.browsers, &hosts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("collect: {e}");
            return ExitCode::FAILURE;
        }
    };
    snap.required = state.required(&cfg.hosts);
    let r = diagnose::analyze(&snap, &cfg.vpn_adapter_keywords, net::best_route);
    println!("config: {}", paths.config.display());
    println!(
        "browser connections: {}, watched hosts: {} (resolved {})",
        snap.conns.len(),
        hosts.len(),
        snap.hosts.len()
    );
    print_report(&r);
    if verbose {
        print_details(&r, cfg);
    }
    match r.status() {
        Status::Stuck | Status::MissingRoute => ExitCode::from(3),
        _ => ExitCode::SUCCESS,
    }
}

fn print_details(r: &Report, cfg: &Config) {
    println!("-- browser connections");
    for (c, rt, kind) in &r.conns {
        let route = rt.map_or("?".into(), |r| {
            format!("/{} if={} src={:?}", r.prefix_len, r.if_index, r.source)
        });
        println!(
            "{} pid={} {} -> {}:{}  {route}  {kind:?}",
            c.process, c.pid, c.local, c.remote, c.remote_port
        );
    }
    println!("-- watched hosts");
    for (h, ips, kind) in &r.hosts {
        println!("{h} {ips:?} {kind:?}");
    }
    println!("-- network services");
    match net::network_services(&net::watched(&cfg.browsers)) {
        Ok(ps) => ps.iter().for_each(|p| println!("{} pid={}", p.name, p.pid)),
        Err(e) => println!("error: {e}"),
    }
}

/// The raw network state a bug report needs (read-only).
pub fn dump() {
    println!(
        "Re-Link {VERSION} on {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let cmds: &[&[&str]] = if cfg!(windows) {
        &[&["route", "print", "-4"], &["ipconfig", "/all"]]
    } else {
        &[
            &["uname", "-a"],
            &["ip", "-4", "addr", "show"],
            &["ip", "-4", "route", "show", "table", "all"],
            &["ip", "rule", "show"],
            &[
                "sh",
                "-c",
                "ps -eo pid,comm,args | grep -i -E 'forti|vpn|ppp' | grep -v grep",
            ],
            &[
                "sh",
                "-c",
                "ls -l /usr/local/bin/relink-helper /etc/sudoers.d/relink 2>&1; sudo -n /usr/local/bin/relink-helper probe 2>&1",
            ],
        ]
    };
    for c in cmds {
        println!("\n$ {}", c.join(" "));
        match std::process::Command::new(c[0]).args(&c[1..]).output() {
            Ok(o) => print!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => println!("({e})"),
        }
    }
    println!();
}

fn print_report(r: &Report) {
    println!("status: {}", ui::status_text(Some(r.status()), Some(r)));
    match &r.vpn {
        Some(v) => println!("vpn: {} (if {}) {:?}", v.description, v.index, v.addrs),
        None => println!("vpn: down"),
    }
    for s in &r.stuck {
        println!(
            "stuck: {} pid={} {} -> {}:{} (route /{} via if {}, source {:?})",
            s.conn.process,
            s.conn.pid,
            s.conn.local,
            s.conn.remote,
            s.conn.remote_port,
            s.route.prefix_len,
            s.route.if_index,
            s.route.source
        );
    }
    for m in &r.missing {
        println!(
            "missing: {} {} routes via if {} (/{}), not the VPN",
            m.host, m.ip, m.route.if_index, m.route.prefix_len
        );
    }
}

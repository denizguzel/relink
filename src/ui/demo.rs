//! Sample states for headless snapshots (`--snap out.png --scene <name>`)
//! and UI tests. Addresses mirror a real FortiClient split tunnel.

use std::collections::{BTreeMap, HashSet};
use std::net::Ipv4Addr;

use super::{HostRow, Level, Page, Published, Sources, Ui};
use crate::diagnose::{Adapter, Conn, Route, Snapshot, analyze};

const HOME: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 108);
const VPN: Ipv4Addr = Ipv4Addr::new(10, 34, 33, 26);
const ALB: [Ipv4Addr; 3] = [
    Ipv4Addr::new(52, 50, 137, 113),
    Ipv4Addr::new(18, 203, 210, 9),
    Ipv4Addr::new(54, 194, 144, 210),
];
const OLD: Ipv4Addr = Ipv4Addr::new(34, 253, 124, 119);

fn route(ip: Ipv4Addr) -> Option<Route> {
    if ALB.contains(&ip) || ip.octets()[0] == 65 {
        Some(Route {
            if_index: 20,
            prefix_len: 32,
            source: Some(VPN),
        })
    } else {
        Some(Route {
            if_index: 7,
            prefix_len: 0,
            source: Some(HOME),
        })
    }
}

fn conn(local: Ipv4Addr, remote: [u8; 4]) -> Conn {
    Conn {
        pid: 23388,
        process: "chrome.exe".into(),
        local,
        local_port: 51234,
        remote: remote.into(),
        remote_port: 443,
    }
}

fn publish(ui: &mut Ui, stuck: bool, missing: bool, vpn_up: bool) {
    ui.cfg.welcomed = true;
    let now = crate::config::now_unix();
    let mut conns = vec![
        conn(VPN, [52, 50, 137, 113]),
        conn(VPN, [65, 9, 9, 89]),
        conn(HOME, [142, 250, 153, 188]),
        conn(HOME, [34, 199, 151, 82]),
    ];
    if stuck {
        conns.push(conn(HOME, [52, 50, 137, 113]));
        conns.push(Conn {
            pid: 36960,
            process: "msedge.exe".into(),
            ..conn(HOME, [18, 203, 210, 9])
        });
    }
    let mut hosts = BTreeMap::from([
        ("app.etycloudbss.com".to_string(), ALB.to_vec()),
        ("app-t6.etycloudbss.com".to_string(), ALB.to_vec()),
        ("logs.etycloudbss.com".to_string(), ALB.to_vec()),
        ("app.etiyacloud.com".to_string(), ALB.to_vec()),
        (
            "wiki.etiya.com".to_string(),
            vec![Ipv4Addr::new(172, 22, 199, 42)],
        ),
        (
            "portal.etiya.com".to_string(),
            vec![Ipv4Addr::new(91, 93, 154, 139)],
        ),
    ]);
    if missing {
        hosts.insert("dev.etycloudbss.com".into(), vec![ALB[0], OLD]);
    }
    let required: HashSet<String> = hosts
        .keys()
        .filter(|h| h.contains("etycloudbss") || h.contains("etiyacloud"))
        .cloned()
        .collect();
    let snap = Snapshot {
        conns,
        adapters: vec![
            Adapter {
                index: 7,
                description: "Intel(R) Ethernet Controller (3) I225-V".into(),
                up: true,
                addrs: vec![HOME],
            },
            Adapter {
                index: 20,
                description: "Fortinet SSL VPN Virtual Ethernet Adapter".into(),
                up: vpn_up,
                addrs: vec![VPN],
            },
        ],
        hosts: hosts.clone(),
        required,
    };
    let report = analyze(&snap, &["Fortinet SSL VPN".into()], route);
    let kinds: BTreeMap<String, _> = report.hosts.iter().map(|h| (h.0.clone(), h.2)).collect();
    let mut rows: Vec<HostRow> = hosts
        .iter()
        .map(|(h, ips)| HostRow {
            host: h.clone(),
            ips: ips.clone(),
            routes: ips
                .iter()
                .map(|ip| (*ip, report.route_of.get(ip).copied().flatten()))
                .collect(),
            kind: kinds.get(h).copied(),
            sources: Sources {
                bookmark: true,
                config: h.starts_with("dev."),
                learned: kinds.get(h) == Some(&crate::diagnose::HostKind::Vpn),
            },
        })
        .collect();
    rows.push(HostRow {
        host: "ci.etiya.com".into(),
        ips: vec![],
        routes: vec![],
        kind: None,
        sources: Sources {
            bookmark: true,
            ..Default::default()
        },
    });
    ui.published = Some(Published {
        report,
        hosts: rows,
        learned: 44,
        checked_at: now - 12,
        helper: cfg!(target_os = "linux"),
    });
    ui.now = now;
    ui.last_fix = Some(now - 600);
    ui.version = env!("CARGO_PKG_VERSION");
    ui.events.clear();
    for (ago, text, level) in [
        (600, "Network service restarted (1 process)", Level::Good),
        (
            607,
            "Notification shown: stuck connection found",
            Level::Warn,
        ),
        (607, "Status: stuck connections: 2", Level::Warn),
        (1200, "Status: connections healthy", Level::Info),
        (1210, "Re-Link 0.2.0 started", Level::Info),
        (
            86_400 + 3000,
            "Network service restarted (1 process)",
            Level::Good,
        ),
        (86_400 + 3300, "Status: stuck connections: 1", Level::Warn),
        (5 * 86_400, "Re-Link 0.1.0 started", Level::Info),
    ] {
        ui.events
            .push_back(super::Event::new(now - ago, text.into(), level));
    }
}

fn tr() {
    crate::i18n::apply("tr");
}

/// A demo scene: its name and how it fills the window state.
pub type Scene = (&'static str, fn(&mut Ui));

pub const SCENES: &[Scene] = &[
    ("update", |u| {
        tr();
        publish(u, false, false, true);
        u.update = Some(crate::update::Available {
            version: "0.3.0".into(),
            url: "https://github.com/denizguzel/relink/releases/tag/v0.3.0".into(),
            download: Some(crate::update::Download {
                name: "relink-0.3.0-windows-x64.exe".into(),
                asset_url: "https://github.com/denizguzel/relink/releases/download/v0.3.0/relink-0.3.0-windows-x64.exe".into(),
                sums_url: "https://github.com/denizguzel/relink/releases/download/v0.3.0/SHA256SUMS".into(),
            }),
        });
    }),
    ("ok", |u| {
        tr();
        publish(u, false, false, true)
    }),
    ("stuck", |u| {
        tr();
        publish(u, true, false, true)
    }),
    ("missing", |u| {
        tr();
        publish(u, false, true, true)
    }),
    ("settling", |u| {
        tr();
        publish(u, false, true, true);
        if let Some(p) = u.published.as_mut() {
            p.report.settling = true;
            p.report.settle_total = 4;
        }
    }),
    ("welcome", |u| {
        tr();
        publish(u, false, false, true);
        u.cfg.welcomed = false;
    }),
    ("paused", |u| {
        tr();
        publish(u, false, false, true);
        u.cfg.welcomed = true;
        u.paused_until = Some(u.now + 42 * 60);
    }),
    ("host-detail", |u| {
        tr();
        publish(u, false, true, true);
        u.page = Page::Hosts;
        u.expanded_host = Some("dev.etycloudbss.com".into());
    }),
    ("vpn-down", |u| {
        tr();
        publish(u, false, false, false)
    }),
    ("connections", |u| {
        tr();
        publish(u, true, false, true);
        u.page = Page::Connections
    }),
    ("hosts", |u| {
        tr();
        publish(u, false, true, true);
        u.page = Page::Hosts
    }),
    ("settings", |u| {
        tr();
        publish(u, false, false, true);
        u.page = Page::Settings
    }),
    ("activity", |u| {
        tr();
        publish(u, true, false, true);
        u.page = Page::Activity
    }),
    ("stuck-en", |u| {
        crate::i18n::apply("en");
        publish(u, true, false, true)
    }),
    ("settings-en", |u| {
        crate::i18n::apply("en");
        publish(u, false, false, true);
        u.page = Page::Settings
    }),
];

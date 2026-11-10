//! What is wrong, per browser and per host. While the VPN's routes arrive the
//! lists give way to one short line, so the page is not a wall of noise.

use rune::{El, col, row, text};

use super::Ctx;
use crate::t;
use crate::ui::parts::{panel, section};

pub(super) fn problems(cx: &Ctx) -> Vec<El> {
    if cx.waiting() {
        return vec![waiting_line(cx)];
    }
    let mut out = Vec::new();
    out.extend(stuck_panel(cx));
    out.extend(missing_panel(cx));
    out
}

/// "Waiting for: a, b, c and 7 more".
fn waiting_line(cx: &Ctx) -> El {
    let names = cx.report.map(|r| r.missing_hosts()).unwrap_or_default();
    let (shown, more) = waiting_summary(&names);
    let line = if more > 0 {
        t!("settle_waiting_more", names = shown, count = more)
    } else {
        t!("settle_waiting", names = shown)
    };
    text(line)
        .key("waiting")
        .font(cx.k.ty.small)
        .color(cx.k.c.text_muted)
        .wrap()
        .w_full()
}

/// One row per stuck connection, with what can be done about it: that depends
/// on the browser (Firefox keeps its sockets in the main process, the others
/// have a network service).
fn stuck_panel(cx: &Ctx) -> Vec<El> {
    let (k, c) = (cx.k, &cx.k.c);
    let Some(r) = cx.report.filter(|r| !r.stuck.is_empty()) else {
        return Vec::new();
    };
    let rows = r
        .stuck
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let note = if crate::diagnose::is_firefox(&st.conn.process) && !cx.helper {
                t!("stuck_note_firefox")
            } else {
                t!("stuck_note_fixable")
            };
            row()
                .key(format!("stuck-{i}"))
                .w_full()
                .px(16.0)
                .py(11.0)
                .gap(16.0)
                .child(
                    text(format!("{} · {}", st.conn.process, st.conn.pid))
                        .font(k.ty.ui)
                        .color(c.text_muted)
                        .w(170.0),
                )
                .child(
                    col()
                        .grow(1.0)
                        .shrink()
                        .gap(4.0)
                        .child(
                            text(format!(
                                "{} → {}:{}",
                                st.conn.local, st.conn.remote, st.conn.remote_port
                            ))
                            .font(k.ty.ui_medium)
                            .color(c.text_base),
                        )
                        .child(
                            text(note)
                                .font(k.ty.small)
                                .color(c.text_muted)
                                .wrap()
                                .w_full(),
                        ),
                )
        })
        .collect();
    vec![section(k, t!("kind_stuck")), panel("stuck", k, rows)]
}

/// One row per host with its addresses beside it; long names wrap.
fn missing_panel(cx: &Ctx) -> Vec<El> {
    let (k, c) = (cx.k, &cx.k.c);
    let Some(r) = cx.report.filter(|r| !r.missing.is_empty()) else {
        return Vec::new();
    };
    let by_host = group_by_host(&r.missing);
    let rows = by_host
        .into_iter()
        .enumerate()
        .map(|(i, (host, ips))| {
            row()
                .key(format!("missing-{i}"))
                .w_full()
                .px(16.0)
                .py(10.0)
                .gap(16.0)
                .center_y()
                .child(
                    text(host)
                        .font(k.ty.ui)
                        .color(c.text_base)
                        .grow(1.0)
                        .shrink()
                        .wrap(),
                )
                .child(
                    text(ips.join("  "))
                        .font(k.ty.mono)
                        .color(c.text_muted)
                        .w(200.0)
                        .wrap(),
                )
        })
        .collect();
    vec![section(k, t!("kind_missing")), panel("missing", k, rows)]
}

/// The first three names, comma-separated, and how many more there are.
fn waiting_summary(names: &[String]) -> (String, usize) {
    (
        names.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
        names.len().saturating_sub(3),
    )
}

/// One entry per host, in first-seen order, with all its missing addresses.
fn group_by_host(missing: &[crate::diagnose::Missing]) -> Vec<(String, Vec<String>)> {
    let mut by_host: Vec<(String, Vec<String>)> = Vec::new();
    for m in missing {
        match by_host.iter_mut().find(|(h, _)| *h == m.host) {
            Some((_, ips)) => ips.push(m.ip.to_string()),
            None => by_host.push((m.host.clone(), vec![m.ip.to_string()])),
        }
    }
    by_host
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::diagnose::{Missing, Route};

    fn missing(host: &str, last: u8) -> Missing {
        Missing {
            host: host.into(),
            ip: Ipv4Addr::new(10, 0, 0, last),
            route: Route {
                if_index: 7,
                prefix_len: 0,
                source: None,
            },
        }
    }

    #[test]
    fn summary_names_three_and_counts_the_rest() {
        let names: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(waiting_summary(&names), ("a, b, c".to_string(), 2));
        assert_eq!(waiting_summary(&names[..2]), ("a, b".to_string(), 0));
        assert_eq!(waiting_summary(&[]), (String::new(), 0));
    }

    #[test]
    fn missing_addresses_are_grouped_per_host_in_order() {
        let grouped = group_by_host(&[
            missing("b.example", 1),
            missing("a.example", 2),
            missing("b.example", 3),
        ]);
        assert_eq!(
            grouped,
            vec![
                (
                    "b.example".to_string(),
                    vec!["10.0.0.1".to_string(), "10.0.0.3".to_string()]
                ),
                ("a.example".to_string(), vec!["10.0.0.2".to_string()]),
            ]
        );
        assert!(group_by_host(&[]).is_empty());
    }
}

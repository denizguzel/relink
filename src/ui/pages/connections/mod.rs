//! The Connections page: browser connections grouped by browser, filterable.

mod group;
mod names;

use rune::{El, Tx, col, el, row, text};
use rune_kit::{Kit, SegmentedControl};

use super::pills::kind_pill;
use crate::t;
use crate::ui::Ui;
use crate::ui::parts::{self, page, panel, section};

pub fn connections(s: &Ui, k: &Kit) -> El {
    let c = &k.c;
    let all: Vec<_> = s
        .published
        .as_ref()
        .map(|p| p.report.conns.clone())
        .unwrap_or_default();
    let groups = group::group(all, s.conn_filter);
    let hosts = s
        .published
        .as_ref()
        .map(|p| p.hosts.as_slice())
        .unwrap_or_default();

    let filter = SegmentedControl::new(
        &[
            t!("filter_all"),
            t!("kind_stuck"),
            t!("kind_vpn"),
            t!("kind_internet"),
        ],
        Some(s.conn_filter),
    )
    .label(t!("connections_title"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| s.conn_filter = i)
    .build(k)
    .key("conn-filter");

    let mut content = vec![filter];
    let mut n = 0;
    if groups.is_empty() {
        content.push(panel(
            "conns",
            k,
            vec![parts::empty(k, t!("connections_empty"))],
        ));
    }
    for (gi, (name, conns)) in groups.iter().enumerate() {
        let rows: Vec<El> = conns
            .iter()
            .map(|(conn, rt, kind)| {
                n += 1;
                let route = rt.map_or("—".to_string(), |r| match r.prefix_len {
                    0 => format!("default · if {}", r.if_index),
                    p => format!("/{p} · if {}", r.if_index),
                });
                row()
                    .key(format!("conn-{n}"))
                    .w_full()
                    .px(16.0)
                    .py(10.0)
                    .gap(14.0)
                    .center_y()
                    .child(el().w(96.0).child(kind_pill(*kind, k)))
                    .child(
                        text(format!("PID {}", conn.pid))
                            .font(k.ty.small)
                            .color(c.text_faint)
                            .w(72.0),
                    )
                    .child(address(
                        conn,
                        names::label(&names::hosts_of(conn.remote, hosts)),
                        k,
                    ))
                    .child(text(route).font(k.ty.small).color(c.text_muted))
            })
            .collect();
        content.push(section(k, format!("{name} · {}", conns.len())));
        content.push(panel(format!("conns-{gi}"), k, rows));
    }
    page(
        "connections",
        k,
        t!("connections_title"),
        Some(t!("connections_hint")),
        content,
    )
}

/// The connection's endpoints, with the host's name under them when known.
fn address(conn: &crate::diagnose::Conn, name: Option<String>, k: &Kit) -> El {
    let c = &k.c;
    let mut cell = col().grow(1.0).shrink().gap(2.0).child(
        text(format!(
            "{}  →  {}:{}",
            conn.local, conn.remote, conn.remote_port
        ))
        .font(k.ty.mono)
        .color(c.text_base),
    );
    if let Some(name) = name {
        cell = cell.child(text(name).font(k.ty.small).color(c.text_muted));
    }
    cell
}

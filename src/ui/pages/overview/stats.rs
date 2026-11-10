//! The four counters under the hero.

use rune::{El, col, row, text};

use super::Ctx;
use crate::diagnose::ConnKind;
use crate::t;
use crate::ui::parts::STAT_VALUE;

pub(super) fn stats(cx: &Ctx) -> El {
    let (k, c) = (cx.k, &cx.k.c);
    let stat = |key: &str, value: usize, label: String| {
        col()
            .key(key.to_string())
            .grow(1.0)
            .shrink()
            .padding(14.0)
            .gap(2.0)
            .radius(10.0)
            .fill(c.layer_01)
            .border(1.0, c.border_muted)
            .child(text(value.to_string()).font(STAT_VALUE).color(c.text_base))
            .child(
                text(label)
                    .font(k.ty.small)
                    .color(c.text_muted)
                    .wrap()
                    .w_full(),
            )
    };
    let published = cx.s.published.as_ref();
    let conns = cx.report.map_or(0, |r| r.conns.len());
    let via_vpn = cx.report.map_or(0, |r| {
        r.conns.iter().filter(|c| c.2 == ConnKind::Vpn).count()
    });
    row()
        .key("stats")
        .w_full()
        .gap(10.0)
        .child(stat("stat-conns", conns, t!("stat_browser_conns")))
        .child(stat("stat-vpn", via_vpn, t!("stat_via_vpn")))
        .child(stat(
            "stat-hosts",
            published.map_or(0, |p| p.hosts.len()),
            t!("stat_watched"),
        ))
        .child(stat(
            "stat-learned",
            published.map_or(0, |p| p.learned),
            t!("stat_learned"),
        ))
}

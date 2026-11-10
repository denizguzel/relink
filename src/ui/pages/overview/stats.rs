//! The connection counters: one thin strip, not three cards.

use rune::{El, el, row, text};

use super::Ctx;
use crate::diagnose::ConnKind;
use crate::t;
use crate::ui::parts::STAT_VALUE;

pub(super) fn stats(cx: &Ctx) -> El {
    let (k, c) = (cx.k, &cx.k.c);
    let cell = |key: &str, value: usize, label: String| {
        row()
            .key(key.to_string())
            .grow(1.0)
            .shrink()
            .px(14.0)
            .py(12.0)
            .gap(8.0)
            .center_y()
            .child(text(value.to_string()).font(STAT_VALUE).color(c.text_base))
            .child(
                text(label)
                    .font(k.ty.small)
                    .color(c.text_muted)
                    .grow(1.0)
                    .shrink()
                    .wrap(),
            )
    };
    let published = cx.s.published.as_ref();
    let conns = cx.report.map_or(0, |r| r.conns.len());
    let via_vpn = cx.report.map_or(0, |r| {
        r.conns.iter().filter(|c| c.2 == ConnKind::Vpn).count()
    });
    let rule = |i: usize| {
        el().key(format!("stat-rule-{i}"))
            .w(1.0)
            .h(28.0)
            .fill(c.border_muted)
    };
    row()
        .key("stats")
        .w_full()
        .center_y()
        .radius(10.0)
        .fill(c.layer_01)
        .border(1.0, c.border_muted)
        .clip_rounded(10.0)
        .child(cell("stat-conns", conns, t!("stat_browser_conns")))
        .child(rule(0))
        .child(cell("stat-vpn", via_vpn, t!("stat_via_vpn")))
        .child(rule(1))
        .child(cell(
            "stat-hosts",
            published.map_or(0, |p| p.hosts.len()),
            t!("stat_watched"),
        ))
}

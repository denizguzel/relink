//! The Hosts page: every watched host with its route and where it came from.
//! Clicking a host opens why it is in its state.

mod detail;
mod explain;
mod list;

use rune::{El, col, el, row, text};
use rune_kit::{Kit, TextInput};

pub(super) use explain::Path;

use super::pills::host_pill;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::{self, page, panel};
use crate::ui::{HostRow, Ui};

/// The source tags of a host: bookmark, setting, learned.
fn tags(h: &HostRow, k: &Kit) -> El {
    let mut tags = row().gap(4.0);
    for (on, label) in [
        (h.sources.bookmark, t!("source_bookmark")),
        (h.sources.config, t!("source_config")),
        (h.sources.learned, t!("source_learned")),
    ] {
        if on {
            tags = tags.child(rune_kit::Badge::new(label).build(k));
        }
    }
    tags
}

/// One host's row; pressing it opens or closes its detail.
fn host_row(i: usize, h: &HostRow, open: bool, k: &Kit) -> El {
    let c = &k.c;
    let ips = if h.ips.is_empty() {
        "—".to_string()
    } else {
        h.ips
            .iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<_>>()
            .join("  ")
    };
    let name = h.host.clone();
    row()
        .key(format!("host-{i}"))
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(14.0)
        .center_y()
        .on_press(move |s: &mut Ui| s.expanded_host = list::toggled(&s.expanded_host, &name))
        .child(el().w(96.0).child(host_pill(h.kind, k)))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(4.0)
                .child(
                    text(h.host.clone())
                        .font(k.ty.ui_medium)
                        .color(c.text_base)
                        .wrap()
                        .w_full(),
                )
                .child(tags(h, k)),
        )
        .child(
            text(ips)
                .font(k.ty.mono)
                .color(c.text_muted)
                .w(200.0)
                .wrap(),
        )
        .child(chevron(open, k))
}

pub fn hosts(s: &Ui, k: &Kit) -> El {
    let all = s
        .published
        .as_ref()
        .map(|p| p.hosts.as_slice())
        .unwrap_or_default();
    let vpn_index = s
        .published
        .as_ref()
        .and_then(|p| p.report.vpn.as_ref())
        .map(|a| a.index);
    let shown = list::visible(all, &s.filter.text);
    let search = TextInput::new(&s.filter)
        .placeholder(t!("filter_placeholder"))
        .label(t!("filter_placeholder"))
        .leading_icon(&icons::SEARCH)
        .clear_button()
        .on_edit(|s: &mut Ui| &mut s.filter)
        .build(k)
        .key("filter");
    let mut rows: Vec<El> = Vec::new();
    for (i, h) in shown.iter().enumerate() {
        let open = s.expanded_host.as_deref() == Some(h.host.as_str());
        rows.push(host_row(i, h, open, k));
        if open {
            rows.push(detail::detail(format!("host-detail-{i}"), h, vpn_index, k));
        }
    }
    if rows.is_empty() {
        rows.push(parts::empty(k, t!("hosts_empty")));
    }
    page(
        "hosts",
        k,
        t!("hosts_title"),
        Some(t!("hosts_hint")),
        vec![search, panel("hosts-list", k, rows)],
    )
}

/// The open/closed marker at the end of a host row.
fn chevron(open: bool, k: &Kit) -> El {
    let icon = if open {
        &icons::CHEVRON_DOWN
    } else {
        &icons::CHEVRON_RIGHT
    };
    el().size(20.0, 20.0)
        .center()
        .child(rune::icon(icon, 16.0).color(k.c.icon_muted))
        .a11y_hidden()
}

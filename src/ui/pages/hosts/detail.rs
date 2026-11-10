//! The expanded part of a host row: what each address does and why the host
//! is in its state.

use rune::{El, col, el, row, text};
use rune_kit::Kit;

use super::explain::{self, Path, Why};
use crate::t;
use crate::ui::HostRow;
use crate::ui::pages::pills::path_pill;

fn why_text(h: &HostRow, vpn_index: Option<u32>) -> String {
    match explain::why(h.kind) {
        Why::Vpn => t!("why_vpn"),
        Why::Missing => t!(
            "why_missing",
            count = explain::home_count(&h.routes, vpn_index)
        ),
        Why::Internet => t!("why_internet"),
        Why::NoVpn => t!("why_no_vpn"),
        Why::Unresolved => t!("why_unresolved"),
    }
}

pub(super) fn detail(key: String, h: &HostRow, vpn_index: Option<u32>, k: &Kit) -> El {
    let c = &k.c;
    let lines: Vec<El> = h
        .routes
        .iter()
        .map(|(ip, route)| {
            let path: Path = explain::path(*route, vpn_index);
            row()
                .key(format!("{key}-{ip}"))
                .gap(12.0)
                .center_y()
                .child(el().w(110.0).child(path_pill(path, k)))
                .child(
                    text(ip.to_string())
                        .font(k.ty.mono)
                        .color(c.text_base)
                        .w(130.0),
                )
                .child(
                    text(explain::route_text(*route))
                        .font(k.ty.small)
                        .color(c.text_muted),
                )
        })
        .collect();
    col()
        .key(key)
        .w_full()
        .px(16.0)
        .pb(12.0)
        .pt(2.0)
        .gap(8.0)
        .child(
            text(why_text(h, vpn_index))
                .font(k.ty.small)
                .color(c.text_muted)
                .wrap()
                .w_full(),
        )
        .children(lines)
}

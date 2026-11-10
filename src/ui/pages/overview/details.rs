//! The VPN adapter, last check and last fix.

use rune::El;

use super::Ctx;
use crate::t;
use crate::ui::Ui;
use crate::ui::parts::{info_row, panel};

/// "12 s ago", "3 min ago" or "Never".
fn ago(s: &Ui, at: Option<u64>) -> String {
    match at {
        None => t!("never"),
        Some(at) => {
            let d = s.now.saturating_sub(at);
            if d < 90 {
                t!("seconds_ago", n = d)
            } else {
                t!("minutes_ago", n = d / 60)
            }
        }
    }
}

/// "chrome.exe · 3 min ago", or just "Never".
fn last_fix_text(s: &Ui) -> String {
    let when = ago(s, s.last_fix);
    if s.last_fix.is_some() && !s.last_fix_what.is_empty() {
        format!("{} · {when}", s.last_fix_what)
    } else {
        when
    }
}

pub(super) fn details(cx: &Ctx) -> El {
    let vpn = match cx.report.and_then(|r| r.vpn.as_ref()) {
        Some(a) => format!(
            "{} · {}",
            a.description,
            a.addrs
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None => t!("vpn_none"),
    };
    let checked_at = cx.s.published.as_ref().map(|p| p.checked_at);
    panel(
        "details",
        cx.k,
        vec![
            info_row("vpn", cx.k, t!("vpn_adapter"), vpn),
            info_row("last-check", cx.k, t!("last_check"), ago(cx.s, checked_at)),
            info_row("last-fix", cx.k, t!("last_fix"), last_fix_text(cx.s)),
        ],
    )
}

//! The details group: the counters, the VPN adapter, the last check and fix,
//! and the latest events. Closed by default so the page stays one decision.

use rune::El;

use super::super::activity::event_row;
use super::Ctx;
use crate::t;
use crate::ui::Ui;
use crate::ui::parts::{self, info_row, panel, section};

/// "12 s ago", "3 min ago" or "Never".
pub(super) fn ago(s: &Ui, at: Option<u64>) -> String {
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

fn details(cx: &Ctx) -> El {
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

/// The recent events, always visible so the page is never a lone card.
pub(super) fn recent(cx: &Ctx) -> Vec<El> {
    let k = cx.k;
    if cx.s.events.is_empty() {
        return Vec::new();
    }
    let rows =
        cx.s.events
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, e)| event_row(&format!("recent-{i}"), e, k))
            .collect();
    vec![section(k, t!("recent_activity")), panel("recent", k, rows)]
}

/// The closed group that holds the technical facts of the last check.
pub(super) fn disclosure(cx: &Ctx) -> El {
    parts::disclosure(
        "details",
        cx.k,
        t!("details"),
        cx.s.details_open,
        |s: &mut Ui| s.details_open = !s.details_open,
        vec![details(cx)],
    )
}

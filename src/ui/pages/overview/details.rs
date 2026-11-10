//! The details group: the counters, the VPN adapter, the last check and fix,
//! and the latest events. Closed by default so the page stays one decision.

use rune::El;

use super::super::activity::event_row;
use super::Ctx;
use crate::t;
use crate::ui::Ui;
use crate::ui::parts::{self, info_row, panel, section};

/// How long ago, coarse on purpose: the text changes at most once a minute,
/// so a value refreshed every second does not flicker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ago {
    Never,
    JustNow,
    Minutes(u64),
}

pub(super) fn ago_of(now: u64, at: Option<u64>) -> Ago {
    match at {
        None => Ago::Never,
        Some(at) => match now.saturating_sub(at) {
            d if d < 60 => Ago::JustNow,
            d => Ago::Minutes(d / 60),
        },
    }
}

/// "Just now", "3 min ago" or "Never".
pub(super) fn ago(s: &Ui, at: Option<u64>) -> String {
    match ago_of(s.now, at) {
        Ago::Never => t!("never"),
        Ago::JustNow => t!("just_now"),
        Ago::Minutes(n) => t!("minutes_ago", n = n),
    }
}

/// The clock time of a check, "01:21:55", for the details panel.
fn clock(at: Option<u64>) -> String {
    match at {
        None => t!("never"),
        Some(at) => {
            let (_, _, _, h, m, sec) = crate::platform::local_parts(at);
            format!("{h:02}:{m:02}:{sec:02}")
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
            info_row("last-check", cx.k, t!("last_check"), clock(checked_at)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ago_changes_at_most_once_a_minute() {
        assert_eq!(ago_of(100, None), Ago::Never);
        for d in [0, 1, 30, 59] {
            assert_eq!(ago_of(1000 + d, Some(1000)), Ago::JustNow, "{d}");
        }
        assert_eq!(ago_of(1060, Some(1000)), Ago::Minutes(1));
        assert_eq!(ago_of(1000 + 179, Some(1000)), Ago::Minutes(2));
        assert_eq!(
            ago_of(5, Some(10)),
            Ago::JustNow,
            "a clock step back is not negative"
        );
    }
}

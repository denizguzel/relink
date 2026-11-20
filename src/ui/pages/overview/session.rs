//! The VPN session card: how long the VPN has been connected, since when, and,
//! once Re-Link has learned when sessions end, how much of this one is left.
//! Shown only while the VPN is up.

use rune::{El, col, el, row, spacer, text};

use super::Ctx;
use crate::brand;
use crate::t;
use crate::ui::duration_text;
use crate::ui::icons;
use crate::ui::parts::{STAT_VALUE, rgb};

/// The warning comes this long before the end; the bar turns amber with it.
const ENDING: u64 = 10 * 60;

/// What the card shows, worked out from the clock.
#[derive(Debug, PartialEq)]
struct Session {
    /// Seconds connected.
    up: u64,
    /// Seconds left before the learned end, while it is still ahead.
    left: Option<u64>,
    /// How much of the learned session has gone, 0..=1.
    done: Option<f32>,
    /// Within the last minutes before the learned end.
    ending: bool,
}

fn session(now: u64, since: u64, limit: Option<u64>) -> Session {
    let up = now.saturating_sub(since);
    // Past the learned end, this session is longer than usual: no countdown.
    let ahead = limit.filter(|&l| l > 0 && up < l);
    let left = ahead.map(|l| l - up);
    Session {
        up,
        left,
        done: ahead.map(|l| up as f32 / l as f32),
        ending: left.is_some_and(|l| l <= ENDING),
    }
}

/// "8" or "8.5" hours, as the notification says it.
fn hours(limit: u64) -> String {
    format!("{}", limit.div_ceil(1800) as f64 / 2.0)
}

pub(super) fn card(cx: &Ctx) -> Option<El> {
    let (k, c) = (cx.k, &cx.k.c);
    let p = cx.s.published.as_ref()?;
    p.report.vpn.as_ref()?;
    let since = p.vpn_since?;
    let s = session(cx.s.now, since, p.session_limit);
    let tint = rgb(if s.ending { brand::AMBER } else { brand::GREEN });

    let (_, _, _, h, m, _) = crate::platform::local_parts(since);
    let head = row()
        .key("session-head")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            el().key("session-icon")
                .size(36.0, 36.0)
                .radius(18.0)
                .fill(tint.alpha(0.14))
                .center()
                .child(rune::icon(&icons::CLOCK, 18.0).color(tint))
                .a11y_hidden(),
        )
        .child(
            col()
                .key("session-copy")
                .gap(2.0)
                .child(
                    text(duration_text(s.up))
                        .key("session-up")
                        .font(STAT_VALUE)
                        .color(c.text_base),
                )
                .child(
                    text(t!("vpn_connected_for"))
                        .font(k.ty.small)
                        .color(c.text_muted),
                ),
        )
        .child(spacer())
        .child(
            text(t!("vpn_connected_at", time = format!("{h:02}:{m:02}")))
                .key("session-since")
                .font(k.ty.small)
                .color(c.text_faint),
        );

    let mut card = col()
        .key("session")
        .w_full()
        .px(16.0)
        .py(14.0)
        .gap(12.0)
        .radius(10.0)
        .fill(c.layer_01)
        .border(1.0, c.border_muted)
        .child(head);

    if let (Some(limit), Some(done), Some(left)) = (p.session_limit, s.done, s.left) {
        let track = row()
            .key("session-track")
            .w_full()
            .h(4.0)
            .radius(2.0)
            .fill(c.border_muted)
            .clip_soft(0.0)
            .child(
                el().key("session-fill")
                    .h_full()
                    .radius(2.0)
                    .fill(tint)
                    .w_frac(done.clamp(0.0, 1.0)),
            )
            .a11y_hidden();
        card = card.child(
            col()
                .key("session-limit")
                .w_full()
                .gap(6.0)
                .child(track)
                .child(
                    text(t!(
                        "session_left",
                        hours = hours(limit),
                        left = duration_text(left)
                    ))
                    .font(k.ty.small)
                    .color(if s.ending { tint } else { c.text_muted })
                    .wrap()
                    .w_full(),
                ),
        );
    } else if let Some(limit) = p.session_limit {
        card = card.child(
            text(t!("session_usual", hours = hours(limit)))
                .key("session-limit")
                .font(k.ty.small)
                .color(c.text_muted)
                .wrap()
                .w_full(),
        );
    }
    Some(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: u64 = 3600;

    #[test]
    fn without_a_learned_end_only_the_time_up_shows() {
        let s = session(1000 + 2 * H, 1000, None);
        assert_eq!(
            s,
            Session {
                up: 2 * H,
                left: None,
                done: None,
                ending: false
            }
        );
    }

    #[test]
    fn with_a_learned_end_the_rest_counts_down() {
        let s = session(2 * H, 0, Some(8 * H));
        assert_eq!(s.left, Some(6 * H));
        assert_eq!(s.done, Some(0.25));
        assert!(!s.ending);
        assert!(session(8 * H - ENDING, 0, Some(8 * H)).ending);
    }

    #[test]
    fn past_the_learned_end_there_is_no_countdown() {
        let s = session(9 * H, 0, Some(8 * H));
        assert_eq!((s.left, s.done, s.ending), (None, None, false));
    }

    #[test]
    fn a_clock_step_back_is_not_negative() {
        assert_eq!(session(10, 50, None).up, 0);
    }

    #[test]
    fn hours_read_in_halves() {
        assert_eq!(hours(8 * H), "8");
        assert_eq!(hours(8 * H - 120), "8");
        assert_eq!(hours(8 * H + 30 * 60), "8.5");
    }
}

//! The status card: what is going on, how far along it is, and what to press.

use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant};

use super::Ctx;
use crate::diagnose::Status;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::{HERO_TITLE, status_color};
use crate::ui::{Cmd, Ui, status_text};

fn detail(status: Option<Status>) -> String {
    match status {
        None => String::new(),
        Some(Status::Ok) => t!("detail_ok"),
        Some(Status::VpnDown) => t!("detail_vpn_down"),
        Some(Status::Settling) => t!("detail_settling"),
        Some(Status::Stuck) => t!("detail_stuck"),
        Some(Status::MissingRoute) => t!("detail_missing"),
    }
}

/// The Lucide icon in the status badge.
fn status_icon(status: Option<Status>) -> &'static rune::Icon {
    match status {
        Some(Status::Ok) => &icons::CHECK,
        Some(Status::Stuck) => &icons::TRIANGLE_ALERT,
        Some(Status::MissingRoute) => &icons::LINK_2_OFF,
        Some(Status::VpnDown) => &icons::UNPLUG,
        Some(Status::Settling) | None => &icons::LOADER_CIRCLE,
    }
}

/// Whether there is something to fix, so pressing "Fix now" needs no confirmation.
fn needs_fix(status: Option<Status>) -> bool {
    matches!(status, Some(Status::Stuck | Status::MissingRoute))
}

/// The text of the "Check now" button: it answers at once, says it is running,
/// then that it finished, and goes back to normal after a few seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckLabel {
    Idle,
    Running,
    Done,
}

fn check_label(pressed: u64, now: u64, checked_at: u64) -> CheckLabel {
    if pressed == 0 || now > pressed + 4 {
        CheckLabel::Idle
    } else if checked_at < pressed {
        CheckLabel::Running
    } else {
        CheckLabel::Done
    }
}

/// Fixing restarts the browser's network service. It is offered only when
/// something is actually stuck: restarting it on a healthy window would be a
/// surprise, not a fix.
fn fix_button(cx: &Ctx) -> El {
    Button::new(t!("fix_now"))
        .icon(&icons::WRENCH)
        .variant(ButtonVariant::Contrast)
        .on_press(|s: &mut Ui| s.send(Cmd::FixNow))
        .build(cx.k)
        .key("fix")
}

/// "Check now" answers at once: the label says it is running, then that it
/// finished, and goes back after a few seconds.
fn check_button(cx: &Ctx) -> El {
    let pressed = cx.s.check_pressed_at;
    let checked = cx.s.published.as_ref().map_or(0, |p| p.checked_at);
    let label = match check_label(pressed, cx.s.now, checked) {
        CheckLabel::Idle => t!("check_now"),
        CheckLabel::Running => t!("check_running"),
        CheckLabel::Done => t!("check_done"),
    };
    Button::new(label)
        .icon(&icons::REFRESH_CW)
        .variant(ButtonVariant::Neutral)
        .on_press(|s: &mut Ui| {
            s.check_pressed_at = s.now;
            s.send(Cmd::CheckNow);
        })
        .build(cx.k)
        .key("check")
}

/// While the VPN's routes arrive: how many of the hosts are ready.
fn progress(cx: &Ctx) -> Option<El> {
    let r = cx
        .report
        .filter(|r| r.waiting_for_routes() && r.settle_total > 0)?;
    let done = r.settle_total - r.missing_hosts().len().min(r.settle_total);
    let bar = rune_kit::Progress::new(Some(done as f32 / r.settle_total as f32))
        .label(t!("settle_progress", done = done, total = r.settle_total))
        .build(cx.k);
    Some(el().key("progress").w_full().pt(4.0).child(bar))
}

/// Nothing to fix or check while the VPN is down; nothing to fix while it
/// connects or when only Firefox is stuck (Re-Link cannot close its sockets).
fn actions(cx: &Ctx) -> Option<El> {
    if cx.status == Some(Status::VpnDown) {
        return None;
    }
    let mut r = row().key("actions").gap(8.0).pt(4.0);
    if needs_fix(cx.status) && !cx.unfixable() {
        r = r.child(fix_button(cx));
    }
    Some(
        r.child(check_button(cx))
            .children(super::paused::button(cx)),
    )
}

/// The last check, a quiet line under the status.
fn meta(cx: &Ctx) -> Option<El> {
    let at = cx.s.published.as_ref()?.checked_at;
    Some(
        text(format!(
            "{}: {}",
            t!("last_check"),
            super::details::ago(cx.s, Some(at))
        ))
        .key("meta")
        .font(cx.k.ty.small)
        .color(cx.k.c.text_faint),
    )
}

pub(super) fn hero(cx: &Ctx) -> El {
    let (k, c) = (cx.k, &cx.k.c);
    let accent = status_color(cx.status);
    row()
        .key("hero")
        .w_full()
        .flex_wrap()
        .items_start()
        .padding(20.0)
        .gap(18.0)
        .radius(12.0)
        .fill(accent.alpha(0.08))
        .border(1.0, accent.alpha(0.35))
        .child(
            el().key("badge")
                .size(44.0, 44.0)
                .radius(22.0)
                .fill(accent)
                .center()
                .child(rune::icon(status_icon(cx.status), 24.0).color(rune::Color::WHITE)),
        )
        .child(
            col()
                .key("copy")
                .grow(1.0)
                .min_w(260.0)
                .shrink()
                .gap(8.0)
                .child(
                    text(status_text(cx.status, cx.report))
                        .key("status")
                        .role("heading")
                        .font(HERO_TITLE)
                        .color(c.text_base)
                        .wrap()
                        .w_full(),
                )
                .child(
                    text(detail(cx.status))
                        .key("detail")
                        .font(k.ty.body)
                        .color(c.text_muted)
                        .wrap()
                        .w_full(),
                )
                .children(meta(cx))
                .children(progress(cx))
                .children(actions(cx)),
        )
        .child(el().key("flow-wrap").pt(2.0).child(super::flow::flow(cx)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_problems_offer_the_fix() {
        assert!(needs_fix(Some(Status::Stuck)) && needs_fix(Some(Status::MissingRoute)));
        for quiet in [
            None,
            Some(Status::Ok),
            Some(Status::VpnDown),
            Some(Status::Settling),
        ] {
            assert!(!needs_fix(quiet), "{quiet:?}");
        }
    }

    #[test]
    fn check_button_runs_then_confirms_then_returns() {
        assert_eq!(check_label(0, 100, 90), CheckLabel::Idle, "never pressed");
        assert_eq!(
            check_label(100, 101, 90),
            CheckLabel::Running,
            "no check finished since the press"
        );
        assert_eq!(
            check_label(100, 101, 100),
            CheckLabel::Done,
            "a check finished at or after the press"
        );
        assert_eq!(
            check_label(100, 105, 104),
            CheckLabel::Idle,
            "back to normal after a few seconds"
        );
    }

    #[test]
    fn every_status_has_a_detail_but_the_first_check() {
        assert!(detail(None).is_empty());
        for st in [
            Status::Ok,
            Status::VpnDown,
            Status::Settling,
            Status::Stuck,
            Status::MissingRoute,
        ] {
            assert!(!detail(Some(st)).is_empty(), "{st:?}");
        }
    }

    #[test]
    fn each_state_has_its_own_badge_icon() {
        let name = |s| status_icon(s).name;
        let all = [
            name(Some(Status::Ok)),
            name(Some(Status::Stuck)),
            name(Some(Status::MissingRoute)),
            name(Some(Status::VpnDown)),
            name(Some(Status::Settling)),
        ];
        let unique: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len(), "{all:?}");
        assert_eq!(
            name(None),
            name(Some(Status::Settling)),
            "checking looks like connecting"
        );
    }
}

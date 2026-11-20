//! "Check now" and what it found. The button answers at once (running, then
//! done), and for a few seconds the line under the status says what was
//! looked at and what came out, so pressing it visibly does something.

use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant};

use super::Ctx;
use crate::diagnose::{Report, Status};
use crate::t;
use crate::ui::icons;
use crate::ui::{Cmd, Ui};

/// How long the button says "Checked" before going back to normal.
const LABEL_SECS: u64 = 4;
/// How long the result line stays after a check the user asked for.
const RESULT_SECS: u64 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Running,
    Done,
}

fn phase(pressed: u64, now: u64, checked_at: u64) -> Phase {
    if pressed == 0 || now > pressed + LABEL_SECS {
        Phase::Idle
    } else if checked_at < pressed {
        Phase::Running
    } else {
        Phase::Done
    }
}

/// Whether the result of a check the user asked for is still worth showing.
fn shows_result(pressed: u64, now: u64, checked_at: u64) -> bool {
    pressed != 0 && checked_at >= pressed && now <= pressed + RESULT_SECS
}

/// What the check looked at, and what came out. `hosts` counts every
/// watched address, as the counters below do (resolved or not).
fn result_text(r: &Report, hosts: usize, status: Option<Status>) -> String {
    let conns = r.conns.len();
    match status {
        Some(Status::Ok) => t!("check_result_ok", conns = conns, hosts = hosts),
        Some(Status::Stuck) => t!(
            "check_result_stuck",
            conns = conns,
            hosts = hosts,
            count = r.stuck.len()
        ),
        Some(Status::MissingRoute) => t!(
            "check_result_missing",
            conns = conns,
            hosts = hosts,
            count = r.missing_hosts().len()
        ),
        _ => t!("check_result", conns = conns, hosts = hosts),
    }
}

/// The line under the status right after "Check now", or None.
pub(super) fn result(cx: &Ctx) -> Option<El> {
    let (k, c) = (cx.k, &cx.k.c);
    let p = cx.s.published.as_ref()?;
    if !shows_result(cx.s.check_pressed_at, cx.s.now, p.checked_at) {
        return None;
    }
    Some(
        row()
            .key("meta")
            .gap(6.0)
            .center_y()
            .child(rune::icon(&icons::CHECK, 14.0).color(c.text_muted))
            .child(
                text(result_text(&p.report, p.hosts.len(), cx.status))
                    .font(k.ty.small)
                    .color(c.text_muted)
                    .wrap()
                    .shrink(),
            ),
    )
}

pub(super) fn button(cx: &Ctx) -> El {
    let checked = cx.s.published.as_ref().map_or(0, |p| p.checked_at);
    let label = match phase(cx.s.check_pressed_at, cx.s.now, checked) {
        Phase::Idle => t!("check_now"),
        Phase::Running => t!("check_running"),
        Phase::Done => t!("check_done"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_button_runs_then_confirms_then_returns() {
        assert_eq!(phase(0, 100, 90), Phase::Idle, "never pressed");
        assert_eq!(phase(100, 101, 90), Phase::Running, "nothing finished yet");
        assert_eq!(phase(100, 101, 100), Phase::Done, "a check finished");
        assert_eq!(
            phase(100, 105, 104),
            Phase::Idle,
            "back after a few seconds"
        );
    }

    #[test]
    fn the_result_stays_a_little_longer_than_the_label() {
        assert!(!shows_result(0, 100, 100), "never pressed");
        assert!(!shows_result(100, 101, 90), "still running");
        assert!(shows_result(100, 101, 101));
        assert!(shows_result(100, 100 + RESULT_SECS, 101));
        assert!(!shows_result(100, 101 + RESULT_SECS, 101), "gone");
    }

    #[test]
    fn the_result_names_what_was_checked() {
        let r = Report::default();
        for st in [
            Some(Status::Ok),
            Some(Status::Stuck),
            Some(Status::MissingRoute),
            Some(Status::TunnelDead),
            None,
        ] {
            assert!(!result_text(&r, 7, st).is_empty(), "{st:?}");
        }
    }
}

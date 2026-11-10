//! Pausing notifications from the window: the start button and the banner
//! that shows what is left of a running pause.

use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant};

use super::Ctx;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::leading_icon;
use crate::ui::pause::{PAUSE_SECS, minutes_left};
use crate::ui::{Cmd, Ui};

/// Minutes left of the running pause, if there is one.
fn remaining(cx: &Ctx) -> Option<u64> {
    minutes_left(cx.s.paused_until, cx.s.now)
}

/// "Pause 1 hour"; only while no pause is running.
pub(super) fn button(cx: &Ctx) -> Option<El> {
    if remaining(cx).is_some() {
        return None;
    }
    Some(
        Button::new(t!("pause_button"))
            .icon(&icons::PAUSE)
            .variant(ButtonVariant::Ghost)
            .on_press(|s: &mut Ui| s.send(Cmd::Pause(PAUSE_SECS)))
            .build(cx.k)
            .key("pause"),
    )
}

/// "Notifications paused · 42 min left" with a resume button.
pub(super) fn banner(cx: &Ctx) -> Option<El> {
    let minutes = remaining(cx)?;
    let (k, c) = (cx.k, &cx.k.c);
    let resume = Button::new(t!("resume_button"))
        .icon(&icons::PLAY)
        .variant(ButtonVariant::Neutral)
        .on_press(|s: &mut Ui| s.send(Cmd::Resume))
        .build(k)
        .key("resume");
    Some(
        row()
            .key("paused-banner")
            .w_full()
            .px(16.0)
            .py(10.0)
            .gap(12.0)
            .center_y()
            .radius(10.0)
            .fill(c.layer_01)
            .border(1.0, c.border_muted)
            .child(leading_icon(&icons::PAUSE, k))
            .child(
                text(t!("paused_banner", minutes = minutes))
                    .font(k.ty.ui)
                    .color(c.text_muted)
                    .grow(1.0)
                    .shrink()
                    .wrap(),
            )
            .child(resume),
    )
}

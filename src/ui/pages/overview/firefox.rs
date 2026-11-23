//! The Firefox warning: its stuck sockets live in its main process, so only a
//! restart clears them. Loud on purpose, with the steps spelled out.

use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant};

use super::Ctx;
use crate::brand;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::rgb;
use crate::ui::{Cmd, Ui};

pub(super) fn card(cx: &Ctx) -> Option<El> {
    if !cx.firefox_stuck() {
        return None;
    }
    let (k, c) = (cx.k, &cx.k.c);
    let tint = rgb(brand::AMBER);
    let step = |i: usize, s: String| {
        row()
            .key(format!("firefox-step-{i}"))
            .w_full()
            .gap(10.0)
            .child(
                text(format!("{}.", i + 1))
                    .font(k.ty.small)
                    .color(tint)
                    .w(16.0),
            )
            .child(
                text(s)
                    .font(k.ty.small)
                    .color(c.text_base)
                    .grow(1.0)
                    .shrink()
                    .wrap(),
            )
    };
    let head = row()
        .key("firefox-head")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            el().key("firefox-icon")
                .size(36.0, 36.0)
                .radius(18.0)
                .fill(tint.alpha(0.16))
                .center()
                .child(rune::icon(&icons::TRIANGLE_ALERT, 18.0).color(tint))
                .a11y_hidden(),
        )
        .child(
            text(t!("firefox_card_title"))
                .font(k.ty.ui_medium)
                .color(c.text_base)
                .grow(1.0)
                .shrink()
                .wrap(),
        );
    let button = Button::new(t!("firefox_restart"))
        .icon(&icons::REFRESH_CW)
        .variant(ButtonVariant::Contrast)
        .on_press(|s: &mut Ui| s.send(Cmd::RestartFirefox))
        .build(k)
        .key("firefox-restart");
    Some(
        col()
            .key("firefox-card")
            .w_full()
            .gap(12.0)
            .px(16.0)
            .py(16.0)
            .radius(10.0)
            .fill(tint.alpha(0.08))
            .border(1.0, tint.alpha(0.6))
            .child(head)
            .child(
                text(t!("firefox_card_body"))
                    .font(k.ty.small)
                    .color(c.text_muted)
                    .wrap()
                    .w_full(),
            )
            .child(
                col()
                    .key("firefox-steps")
                    .w_full()
                    .gap(6.0)
                    .child(step(0, t!("firefox_card_step_save")))
                    .child(step(1, t!("firefox_card_step_restart")))
                    .child(step(2, t!("firefox_card_step_missing"))),
            )
            .child(row().key("firefox-actions").child(button)),
    )
}

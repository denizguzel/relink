//! The first screen a new user sees: no sidebar, just what Re-Link does and
//! the two choices that matter (how many notifications, whether to start with
//! the session). Dismissing it once shows the normal window from then on.

use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

use super::Ui;
use super::icons;
use super::pages::controls;
use super::parts::{self, HERO_TITLE, panel, setting_row};
use crate::t;

/// Whether to show the screen: until the user dismisses it once.
pub fn wanted(s: &Ui) -> bool {
    !s.cfg.welcomed
}

pub fn screen(s: &Ui, k: &Kit) -> El {
    let c = &k.c;
    let done = Button::new(t!("welcome_start"))
        .icon(&icons::ARROW_RIGHT)
        .variant(ButtonVariant::Contrast)
        .on_press(|s: &mut Ui| s.change(|c| c.welcomed = true))
        .build(k)
        .key("welcome-done");
    let brand = row()
        .key("welcome-brand")
        .gap(10.0)
        .center_y()
        .child(rune::icon(&icons::SHIELD_CHECK, 20.0).color(parts::rgb(crate::iconart::GREEN)))
        .child(text("Re-Link").font(parts::BRAND).color(c.text_base));
    let card = col()
        .key("welcome-card")
        .w(700.0)
        .gap(18.0)
        .child(brand)
        .child(
            col()
                .key("welcome-copy")
                .gap(8.0)
                .child(
                    text(t!("welcome_title"))
                        .role("heading")
                        .font(HERO_TITLE)
                        .color(c.text_base)
                        .wrap()
                        .w_full(),
                )
                .child(
                    text(t!("welcome_body"))
                        .font(k.ty.body)
                        .color(c.text_muted)
                        .wrap()
                        .w_full(),
                ),
        )
        .child(panel(
            "welcome-choices",
            k,
            vec![
                setting_row(
                    "welcome-notify",
                    k,
                    &icons::BELL,
                    t!("notifications"),
                    Some(t!("notifications_desc")),
                    controls::notifications(s, k),
                ),
                setting_row(
                    "welcome-startup",
                    k,
                    &icons::POWER,
                    t!("startup"),
                    Some(t!("startup_desc")),
                    controls::startup(s, k),
                ),
            ],
        ))
        .child(row().key("welcome-actions").child(done));
    col()
        .key("welcome")
        .w_full()
        .h_full()
        .fill(c.bg_base)
        .center()
        .child(el().key("welcome-center").child(card))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shown_until_dismissed() {
        let mut s = Ui::default();
        assert!(wanted(&s));
        s.cfg.welcomed = true;
        assert!(!wanted(&s));
    }
}

//! The in-app help sheet: what Re-Link does, what the tray colours mean, and
//! the diagnostics a bug report needs.

use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, ScrollBar};

use super::Ui;
use super::icons;
use super::parts::{self, panel, section};
use crate::brand;
use crate::t;

/// One line with a leading icon.
fn tip(icon: &'static rune::Icon, text_: String, k: &Kit) -> El {
    row()
        .key(format!("tip-{}", icon.name))
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(12.0)
        .center_y()
        .child(rune::icon(icon, 18.0).color(k.c.icon_muted))
        .child(
            text(text_)
                .font(k.ty.ui)
                .color(k.c.text_base)
                .grow(1.0)
                .shrink()
                .wrap(),
        )
}

/// One tray colour and what it means.
fn tray_row(color: crate::brand::Rgb, label: String, k: &Kit) -> El {
    row()
        .key(format!("tray-{label}"))
        .w_full()
        .px(16.0)
        .py(7.0)
        .gap(10.0)
        .center_y()
        .child(el().size(10.0, 10.0).radius(5.0).fill(parts::rgb(color)))
        .child(text(label).font(k.ty.small).color(k.c.text_muted))
}

pub fn body(s: &Ui, k: &Kit) -> El {
    let c = &k.c;
    let copied = s.copied_at != 0 && s.now <= s.copied_at + 3;
    let copy = Button::new(if copied {
        t!("diagnostics_copied")
    } else {
        t!("copy_diagnostics")
    })
    .icon(&icons::COPY)
    .variant(ButtonVariant::Neutral)
    .on_press_tx(|s: &mut Ui, tx: &mut rune::Tx| {
        s.copied_at = s.now;
        tx.copy(crate::ui::diagnostics(s));
    })
    .build(k)
    .key("help-copy");
    col()
        .key("help-body")
        .w_full()
        .px(16.0)
        .pb(16.0)
        .gap(16.0)
        .child(
            col()
                .key("help-scroll")
                .w_full()
                .max_h(460.0)
                .gap(12.0)
                .child(
                    text(t!("help_lead"))
                        .font(k.ty.ui)
                        .color(c.text_muted)
                        .wrap()
                        .w_full(),
                )
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .scroll_bar(k)
                .child(
                    panel(
                        "help-tips",
                        k,
                        vec![
                            tip(&icons::WRENCH, t!("help_fix"), k),
                            tip(&icons::BELL, t!("help_notify"), k),
                        ],
                    )
                    .mt(14.0),
                )
                .child(section(k, t!("help_tray")).mt(18.0))
                .child(
                    panel(
                        "help-tray",
                        k,
                        vec![
                            tray_row(brand::GREEN, t!("tray_ok"), k),
                            tray_row(brand::GRAY, t!("tray_vpn_down"), k),
                            tray_row(brand::AMBER, t!("tray_stuck"), k),
                            tray_row(brand::RED, t!("tray_missing"), k),
                        ],
                    )
                    .mt(8.0),
                )
                .child(
                    text(t!("manual_tip"))
                        .mt(14.0)
                        .font(k.ty.small)
                        .color(c.text_muted)
                        .wrap()
                        .w_full(),
                ),
        )
        .child(
            row()
                .key("help-footer")
                .w_full()
                .gap(8.0)
                .center_y()
                .child(
                    text(format!("Re-Link v{}", s.version))
                        .font(k.ty.small)
                        .color(c.text_faint)
                        .grow(1.0),
                )
                .child(copy),
        )
}

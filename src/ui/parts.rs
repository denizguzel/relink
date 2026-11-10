//! Shared building blocks: page scaffold, panels, rows, status colours.

use rune::{Color, El, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::{Kit, ScrollBar};

use crate::diagnose::Status;
use crate::iconart;

pub const BRAND: TextStyle = TextStyle::new(15.0, 650.0).lh(20.0);
pub const PAGE_TITLE: TextStyle = TextStyle::new(20.0, 650.0).lh(26.0);
pub const HERO_TITLE: TextStyle = TextStyle::new(22.0, 650.0).lh(28.0);
pub const STAT_VALUE: TextStyle = TextStyle::new(24.0, 600.0).lh(30.0);
pub const GUTTER: f32 = 28.0;

pub fn rgb(c: iconart::Rgb) -> Color {
    rune::color::hex(&format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])).unwrap_or(Color::BLACK)
}

/// The badge colour of a status (blue while the first check runs).
pub fn status_color(status: Option<Status>) -> Color {
    rgb(match status {
        None => iconart::BLUE,
        Some(Status::Ok) => iconart::GREEN,
        Some(Status::VpnDown) => iconart::GRAY,
        Some(Status::Settling) => iconart::BLUE,
        Some(Status::Stuck) => iconart::AMBER,
        Some(Status::MissingRoute) => iconart::RED,
    })
}

pub fn status_dot(status: Option<Status>, size: f32) -> El {
    el().size(size, size)
        .radius(size / 2.0)
        .fill(status_color(status))
        .a11y_hidden()
}

/// A page: title, optional hint, then content, scrolling as one column.
pub fn page(key: &str, k: &Kit, title: String, hint: Option<String>, content: Vec<El>) -> El {
    let mut head = col().key("head").gap(6.0).child(
        text(title)
            .role("heading")
            .font(PAGE_TITLE)
            .color(k.c.text_base),
    );
    if let Some(h) = hint {
        head = head.child(
            text(h)
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full(),
        );
    }
    // The scroll bar follows the system's "Show scroll bars" setting.
    col()
        .key(key)
        .w_full()
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("content")
                .w_full()
                .px(GUTTER)
                .pt(24.0)
                .pb(GUTTER)
                .gap(18.0)
                .child(head)
                .children(content),
        )
        .scroll_bar(k)
}

/// A raised group of rows separated by hairlines.
pub fn panel(key: impl Into<String>, k: &Kit, rows: Vec<El>) -> El {
    let mut p = col()
        .key(key.into())
        .w_full()
        .radius(10.0)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_muted)
        .clip_rounded(10.0);
    let n = rows.len();
    for (i, r) in rows.into_iter().enumerate() {
        p = p.child(r);
        if i + 1 < n {
            p = p.child(
                el().key(format!("rule-{i}"))
                    .h(1.0)
                    .w_full()
                    .fill(k.c.border_muted)
                    .a11y_hidden(),
            );
        }
    }
    p
}

/// A muted label above a panel.
pub fn section(k: &Kit, title: String) -> El {
    text(title)
        .role("heading")
        .font(k.ty.ui_medium)
        .color(k.c.text_muted)
}

/// A settings-style row: title and description on the left, a control on
/// the right.
pub fn setting_row(
    key: &str,
    k: &Kit,
    icon: &'static rune::Icon,
    title: String,
    desc: Option<String>,
    control: El,
) -> El {
    let mut copy = col()
        .key("copy")
        .grow(1.0)
        .shrink()
        .gap(2.0)
        .child(text(title).font(k.ty.ui_medium).color(k.c.text_base));
    if let Some(d) = desc {
        copy = copy.child(
            text(d)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap()
                .w_full(),
        );
    }
    row()
        .key(key.to_string())
        .w_full()
        .px(16.0)
        .py(12.0)
        .gap(14.0)
        .center_y()
        .child(leading_icon(icon, k))
        .child(copy)
        .child(control)
}

/// The muted icon at the start of a row.
pub fn leading_icon(i: &'static rune::Icon, k: &Kit) -> El {
    el().size(20.0, 20.0)
        .center()
        .child(rune::icon(i, 18.0).color(k.c.icon_muted))
        .a11y_hidden()
}

/// A key/value row.
pub fn info_row(key: &str, k: &Kit, label: String, value: String) -> El {
    row()
        .key(key.to_string())
        .w_full()
        .px(16.0)
        .py(11.0)
        .gap(16.0)
        .center_y()
        .child(text(label).font(k.ty.ui).color(k.c.text_muted).w(170.0))
        .child(
            text(value)
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0)
                .shrink()
                .wrap(),
        )
}

/// A small coloured pill (status of a row).
pub fn pill(label: String, color: Color, k: &Kit) -> El {
    row()
        .role("status")
        .label(label.clone())
        .h(20.0)
        .px(8.0)
        .gap(6.0)
        .center()
        .radius(10.0)
        .fill(color.alpha(0.14))
        .child(el().size(6.0, 6.0).radius(3.0).fill(color))
        .child(
            text(label)
                .font(k.ty.small_medium)
                .color(k.c.text_base)
                .a11y_hidden(),
        )
}

/// An empty-state line inside a panel.
pub fn empty(k: &Kit, msg: String) -> El {
    row()
        .w_full()
        .px(16.0)
        .py(18.0)
        .child(text(msg).font(k.ty.body).color(k.c.text_muted))
}

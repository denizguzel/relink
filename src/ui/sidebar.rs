//! The sidebar: brand, page links with badges, quit, and the status footer.

use rune::{El, Tx, col, el, row, text};
use rune_kit::{Kit, Sidebar, SidebarRow};

use super::icons;
use super::parts;
use super::{Page, Ui, short_status};
use crate::t;

/// A small line in the sidebar footer that acts like a button (hover fill,
/// keyboard focus) but reads as plain text.
fn footer_link(k: &Kit, label: String, help: String, child: El) -> El {
    el().role("button")
        .label(label)
        .help(help)
        .focusable(true)
        .focus_ring(2.0, 2.5, k.c.border_focus)
        .py(2.0)
        .radius(4.0)
        .hover(k.c.layer_02)
        .child(child)
}

pub fn sidebar(s: &Ui, k: &Kit) -> El {
    let c = &k.c;
    let report = s.published.as_ref().map(|p| &p.report);
    let status = s.status();

    let nav = |page: Page, icon: &'static rune::Icon, label: String, badge: Option<usize>| {
        let mut r = SidebarRow::new(label)
            .icon(icon)
            .selected(s.page == page)
            .on_select(move |s: &mut Ui, _: &mut Tx| s.page = page);
        if let Some(n) = badge.filter(|n| *n > 0) {
            r = r.badge(n.to_string());
        }
        r.build(k).key(format!("nav-{page:?}"))
    };
    let stuck = report.map_or(0, |r| r.stuck.len());
    let missing = report.map_or(0, |r| r.missing_hosts().len());

    let brand = row()
        .key("brand")
        .gap(10.0)
        .center_y()
        .px(10.0)
        .pt(14.0)
        .pb(10.0)
        .child(parts::logo(k))
        .child(text("Re-Link").font(parts::BRAND).color(c.text_base))
        .child(parts::status_dot(status, 8.0));
    let help = SidebarRow::new(t!("help"))
        .icon(&icons::CIRCLE_HELP)
        .on_select(|s: &mut Ui, _: &mut Tx| s.help_open = true)
        .build(k)
        .key("help");
    let quit = SidebarRow::new(t!("quit"))
        .icon(&icons::POWER)
        .on_select(|_: &mut Ui, tx: &mut Tx| tx.quit())
        .build(k)
        .key("quit");
    let copied = s.version_copied_at != 0 && s.now <= s.version_copied_at + 2;
    let version = format!("v{}", s.version);
    let version_text = if copied {
        t!("version_copied")
    } else {
        version.clone()
    };
    let mut version_line = row().key("sidebar-version").gap(8.0).center_y().child(
        footer_link(
            k,
            version.clone(),
            t!("copy_version"),
            text(version_text).font(k.ty.small).color(if copied {
                c.text_muted
            } else {
                c.text_faint
            }),
        )
        .on_press_tx(move |s: &mut Ui, tx: &mut Tx| {
            s.version_copied_at = s.now;
            tx.copy(format!("v{}", s.version));
        }),
    );
    if let Some(found) = &s.update {
        let hint = t!("update_short", version = found.version);
        version_line = version_line.child(
            footer_link(
                k,
                hint.clone(),
                t!("check_updates"),
                text(hint)
                    .font(k.ty.small)
                    .color(parts::rgb(crate::brand::BLUE)),
            )
            .on_press(|s: &mut Ui| s.page = Page::Settings),
        );
    }
    let footer = col()
        .key("sidebar-footer")
        .pb(14.0)
        .gap(10.0)
        .child(help)
        .child(quit)
        .child(
            col()
                .key("sidebar-status")
                .px(14.0)
                .gap(2.0)
                .child(
                    text(short_status(report))
                        .font(k.ty.small_medium)
                        .color(c.text_muted)
                        .wrap(),
                )
                .child(version_line),
        );
    Sidebar::new()
        .label("Re-Link")
        .header(brand)
        .child(nav(
            Page::Overview,
            &icons::ACTIVITY,
            t!("nav_overview"),
            None,
        ))
        .child(nav(
            Page::Connections,
            &icons::LINK,
            t!("nav_connections"),
            Some(stuck),
        ))
        .child(nav(
            Page::Hosts,
            &icons::SERVER,
            t!("nav_hosts"),
            Some(missing),
        ))
        .child(nav(
            Page::Settings,
            &icons::SLIDERS_HORIZONTAL,
            t!("nav_settings"),
            None,
        ))
        .child(nav(
            Page::Activity,
            &icons::HISTORY,
            t!("nav_activity"),
            None,
        ))
        .footer(footer)
        .build(k)
}

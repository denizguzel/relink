//! The sidebar: brand, page links with badges, quit, and the status footer.

use rune::{El, Tx, col, row, text};
use rune_kit::{Kit, Sidebar, SidebarRow};

use super::icons;
use super::parts;
use super::{Page, Ui, short_status};
use crate::t;

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
        .px(12.0)
        .pt(14.0)
        .pb(10.0)
        .child(parts::status_dot(status, 12.0))
        .child(text("Re-Link").font(parts::BRAND).color(c.text_base));
    let quit = SidebarRow::new(t!("quit"))
        .icon(&icons::POWER)
        .on_select(|_: &mut Ui, tx: &mut Tx| tx.quit())
        .build(k)
        .key("quit");
    let footer = col()
        .key("sidebar-footer")
        .pb(14.0)
        .gap(10.0)
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
                .child(
                    text(format!("v{}", s.version))
                        .font(k.ty.small)
                        .color(c.text_faint),
                ),
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

//! The window's layout: sidebar on the left, the current page on the right.

use rune::{Cx, El, col, row};
use rune_kit::Dialog;

use super::{Page, Ui, help, pages, sidebar::sidebar, theme, welcome, zoom};
use crate::platform::{TitleBar, match_title_bar};
use crate::t;

pub fn view(s: &Ui, cx: &mut Cx<Ui>) -> El {
    let system_dark = cx.system_dark();
    let dark = theme::is_dark(&s.cfg.theme, system_dark);
    let k = rune_theme_opencode::kit(dark);
    let c = &k.c;
    // The system draws the title bar: repaint it only when the theme setting
    // forces the other appearance.
    let [r, g, b, _] = c.bg_deep.premul_srgb(1.0);
    let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    match_title_bar(
        match theme::forced_title_bar(&s.cfg.theme, system_dark) {
            Some(dark) => TitleBar::Forced {
                dark,
                background: [byte(r), byte(g), byte(b)],
            },
            None => TitleBar::System,
        },
        system_dark,
    );

    // A new user sees only the welcome screen, without the sidebar.
    if welcome::wanted(s) {
        return rune_kit::root(
            zoom::wrap(welcome::screen(s, k), s.window_size, s.cfg.zoom_percent),
            Vec::new(),
        );
    }

    let page = match s.page {
        Page::Overview => pages::overview(s, k),
        Page::Connections => pages::connections(s, k),
        Page::Hosts => pages::hosts(s, k),
        Page::Settings => pages::settings(s, k),
        Page::Activity => pages::activity(s, k),
    };

    let body = row()
        .key("shell")
        .w_full()
        .h_full()
        .fill(c.bg_deep)
        .child(col().key("sidebar").w(208.0).h_full().child(sidebar(s, k)))
        .child(
            col()
                .key("panel")
                .grow(1.0)
                .shrink()
                .h_full()
                .mt(8.0)
                .mr(8.0)
                .mb(8.0)
                .radius(12.0)
                .fill(c.bg_base)
                .border(1.0, c.border_muted)
                .clip_rounded(12.0)
                .child(page),
        );
    let mut layers = Vec::new();
    Dialog::new("help", t!("help_title"))
        .width(540.0)
        .fit()
        .open(s.help_open)
        .on_close(|s: &mut Ui| s.help_open = false)
        .child(help::body(s, k))
        .build(k, &mut layers);
    rune_kit::root(zoom::wrap(body, s.window_size, s.cfg.zoom_percent), layers)
}

//! The Overview page: the status hero, counters, details, what is wrong and
//! the latest events. Each part lives in its own module.

mod details;
mod flow;
mod hero;
mod paused;
mod problems;
mod stats;

use rune::{El, col, row};
use rune_kit::Kit;

use crate::diagnose::{Report, Status};
use crate::t;
use crate::ui::Ui;
use crate::ui::parts::page;

/// What every part of the page reads: the window state and the latest report.
struct Ctx<'a> {
    s: &'a Ui,
    k: &'a Kit,
    report: Option<&'a Report>,
    status: Option<Status>,
    /// Whether the Linux root helper can close single sockets.
    helper: bool,
}

impl<'a> Ctx<'a> {
    fn new(s: &'a Ui, k: &'a Kit) -> Ctx<'a> {
        let published = s.published.as_ref();
        Ctx {
            s,
            k,
            report: published.map(|p| &p.report),
            status: s.status(),
            helper: published.is_some_and(|p| p.helper),
        }
    }

    /// Every stuck connection belongs to Firefox, which keeps its sockets in
    /// its main process: nothing Re-Link can close (unless the helper is there).
    fn unfixable(&self) -> bool {
        self.status == Some(Status::Stuck)
            && !self.helper
            && self.report.is_some_and(|r| {
                !r.stuck.is_empty()
                    && r.stuck_processes()
                        .iter()
                        .all(|n| crate::diagnose::is_firefox(n))
            })
    }

    /// The VPN's routes are still arriving, so problems are not news yet.
    fn waiting(&self) -> bool {
        self.report.is_some_and(Report::waiting_for_routes)
    }
}

pub fn overview(s: &Ui, k: &Kit) -> El {
    let cx = Ctx::new(s, k);
    let mut content = vec![hero::hero(&cx)];
    content.extend(paused::banner(&cx));
    // Two columns when the window is wide enough, stacked when it is not.
    let mut main = col().key("main").grow(3.0).min_w(320.0).gap(18.0);
    main = main
        .children(problems::problems(&cx))
        .child(stats::stats(&cx))
        .child(details::disclosure(&cx));
    let mut columns = row().key("columns").w_full().gap(18.0).flex_wrap();
    columns = columns.child(main);
    let recent = details::recent(&cx);
    if !recent.is_empty() {
        columns = columns.child(
            col()
                .key("side")
                .grow(2.0)
                .min_w(280.0)
                .gap(10.0)
                .children(recent),
        );
    }
    content.push(columns);
    page("overview", k, t!("nav_overview"), None, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::demo::SCENES;

    fn scene(name: &str) -> Ui {
        let mut ui = Ui::default();
        (SCENES
            .iter()
            .find(|(n, _)| *n == name)
            .expect("scene exists")
            .1)(&mut ui);
        ui
    }

    #[test]
    fn chrome_and_edge_stuck_connections_can_be_fixed() {
        let (ui, kit) = (scene("stuck"), rune_theme_opencode::kit(true));
        let cx = Ctx::new(&ui, kit);
        assert_eq!(cx.status, Some(Status::Stuck));
        assert!(!cx.unfixable());
    }

    #[test]
    fn firefox_alone_cannot_be_fixed_without_the_helper() {
        let (mut ui, kit) = (scene("stuck"), rune_theme_opencode::kit(true));
        for (c, _, _) in ui.published.as_mut().unwrap().report.conns.iter_mut() {
            c.process = "firefox.exe".into();
        }
        for st in ui.published.as_mut().unwrap().report.stuck.iter_mut() {
            st.conn.process = "firefox.exe".into();
        }
        ui.published.as_mut().unwrap().helper = false;
        assert!(Ctx::new(&ui, kit).unfixable());
        ui.published.as_mut().unwrap().helper = true;
        assert!(
            !Ctx::new(&ui, kit).unfixable(),
            "the root helper can close its sockets"
        );
    }

    #[test]
    fn a_healthy_state_is_never_unfixable_or_waiting() {
        let (ui, kit) = (scene("ok"), rune_theme_opencode::kit(true));
        let cx = Ctx::new(&ui, kit);
        assert!(!cx.unfixable() && !cx.waiting());
    }

    #[test]
    fn routes_arriving_count_as_waiting() {
        let (ui, kit) = (scene("settling"), rune_theme_opencode::kit(true));
        let cx = Ctx::new(&ui, kit);
        assert!(cx.waiting());
        assert_eq!(cx.status, Some(Status::Settling));
    }
}

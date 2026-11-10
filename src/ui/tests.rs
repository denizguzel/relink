//! Tests over whole window states built from the demo scenes.

use super::demo::SCENES;
use super::{Level, MAX_EVENTS, Ui, diagnostics, short_status, status_text};
use crate::diagnose::Status;

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
fn the_event_list_keeps_only_the_newest() {
    let mut ui = Ui::default();
    for i in 0..(MAX_EVENTS + 25) {
        ui.push_event(format!("event {i}"), Level::Info);
    }
    assert_eq!(ui.events.len(), MAX_EVENTS);
    assert_eq!(
        ui.events.front().unwrap().text,
        format!("event {}", MAX_EVENTS + 24),
        "newest first"
    );
    assert_eq!(ui.events.back().unwrap().text, "event 25");
}

#[test]
fn an_event_knows_its_local_day_and_time() {
    let e = super::Event::new(1_794_331_321, "x".into(), Level::Good);
    assert_eq!(e.time.len(), 8, "HH:MM:SS");
    assert!(e.day.0 >= 2026 && (1..=12).contains(&e.day.1) && (1..=31).contains(&e.day.2));
}

#[test]
fn diagnostics_cover_status_vpn_connections_hosts_and_events() {
    let ui = scene("stuck");
    let text = diagnostics(&ui);
    for needle in [
        "Re-Link ",
        "status:",
        "vpn: Fortinet",
        "-- browser connections",
        "chrome.exe",
        "-- watched hosts",
        "-- recent events",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
}

#[test]
fn diagnostics_work_before_the_first_check() {
    let text = diagnostics(&Ui::default());
    assert!(text.contains("status:") && text.contains("-- recent events"));
    assert!(!text.contains("-- browser connections"));
}

#[test]
fn the_vpn_down_diagnostics_say_so() {
    assert!(diagnostics(&scene("vpn-down")).contains("vpn: not connected"));
}

#[test]
fn status_texts_follow_the_status() {
    assert_eq!(scene("stuck").status(), Some(Status::Stuck));
    assert_eq!(scene("ok").status(), Some(Status::Ok));
    assert_eq!(scene("vpn-down").status(), Some(Status::VpnDown));
    assert_eq!(scene("missing").status(), Some(Status::MissingRoute));
    assert_eq!(scene("settling").status(), Some(Status::Settling));
}

#[test]
fn each_status_reads_differently() {
    let text = |name: &str| {
        let ui = scene(name);
        status_text(ui.status(), ui.published.as_ref().map(|p| &p.report))
    };
    let all: Vec<String> = ["ok", "stuck", "missing", "vpn-down", "settling"]
        .iter()
        .map(|n| text(n))
        .collect();
    let unique: std::collections::HashSet<&String> = all.iter().collect();
    assert_eq!(unique.len(), all.len(), "{all:?}");
    assert!(
        text("stuck").contains('2'),
        "the stuck count is part of the text"
    );
}

#[test]
fn the_short_status_is_short_enough_for_the_tray() {
    for name in ["ok", "stuck", "missing", "vpn-down", "settling"] {
        let ui = scene(name);
        let text = short_status(ui.published.as_ref().map(|p| &p.report));
        assert!(text.chars().count() < 100, "{name}: {text}");
    }
}

#[test]
fn only_the_welcome_scenes_show_the_welcome_screen() {
    for (name, apply) in SCENES {
        let mut ui = Ui::default();
        apply(&mut ui);
        assert_eq!(
            super::welcome::wanted(&ui),
            matches!(*name, "welcome" | "welcome-light"),
            "scene {name}"
        );
    }
}

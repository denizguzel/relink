//! The Activity page: events grouped by day, filterable by level.

mod days;

use rune::{El, Tx, row, text};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl};

use crate::iconart;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::{self, page, panel, section};
use crate::ui::{Event, Level, Ui};

/// The Lucide icon of an event's level.
fn level_icon(level: Level) -> &'static rune::Icon {
    match level {
        Level::Info => &icons::INFO,
        Level::Good => &icons::CIRCLE_CHECK,
        Level::Warn => &icons::TRIANGLE_ALERT,
    }
}

pub(super) fn event_row(key: &str, e: &Event, k: &Kit) -> El {
    let color = parts::rgb(match e.level {
        Level::Info => iconart::BLUE,
        Level::Good => iconart::GREEN,
        Level::Warn => iconart::AMBER,
    });
    row()
        .key(key.to_string())
        .w_full()
        .px(16.0)
        .py(9.0)
        .gap(12.0)
        .center_y()
        .child(rune::icon(level_icon(e.level), 16.0).color(color))
        .child(
            text(e.time.clone())
                .font(k.ty.mono)
                .color(k.c.text_faint)
                .w(64.0),
        )
        .child(
            text(e.text.clone())
                .font(k.ty.ui)
                .color(k.c.text_base)
                .grow(1.0)
                .shrink()
                .wrap(),
        )
}

/// The title of a day group, relative to `now`.
fn day_label(day: days::Day, now: u64) -> String {
    let date = |ts| {
        let (y, m, d, ..) = crate::platform::local_parts(ts);
        (y, m, d)
    };
    match days::title(day, date(now), date(now.saturating_sub(86_400))) {
        days::DayTitle::Today => t!("day_today"),
        days::DayTitle::Yesterday => t!("day_yesterday"),
        days::DayTitle::Date(d) => days::format_date(d, crate::i18n::current().starts_with("tr")),
    }
}

pub fn activity(s: &Ui, k: &Kit) -> El {
    let mut content: Vec<El> = Vec::new();
    let filter = SegmentedControl::new(
        &[t!("filter_all"), t!("filter_warnings"), t!("filter_fixes")],
        Some(s.event_filter),
    )
    .label(t!("activity_title"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| s.event_filter = i)
    .build(k)
    .key("event-filter");
    content.push(filter);
    let events = days::filtered(&s.events, s.event_filter);
    if events.is_empty() {
        content.push(panel(
            "events",
            k,
            vec![parts::empty(k, t!("activity_empty"))],
        ));
    } else {
        let mut index = 0;
        for (day, run) in days::by_day(&events) {
            let mut rows = Vec::new();
            for e in &run {
                index += 1;
                rows.push(event_row(&format!("ev-{index}"), e, k));
            }
            content.push(section(k, day_label(day, s.now)));
            content.push(panel(format!("events-{index}"), k, rows));
        }
    }
    let open_log = Button::new(t!("open_log_file"))
        .icon(&icons::FILE_TEXT)
        .variant(ButtonVariant::Neutral)
        .on_press(|s: &mut Ui| {
            if let Some(l) = &s.link {
                crate::platform::open_in_editor(&l.log_path);
            }
        })
        .build(k)
        .key("open-log");
    let copied = s.copied_at != 0 && s.now <= s.copied_at + 3;
    let copy = Button::new(if copied {
        t!("diagnostics_copied")
    } else {
        t!("copy_diagnostics")
    })
    .icon(&icons::COPY)
    .variant(ButtonVariant::Neutral)
    .on_press_tx(|s: &mut Ui, tx: &mut Tx| {
        s.copied_at = s.now;
        tx.copy(crate::ui::diagnostics(s));
    })
    .build(k)
    .key("copy-diagnostics");
    content.push(
        row()
            .key("activity-actions")
            .gap(8.0)
            .child(copy)
            .child(open_log),
    );
    page(
        "activity",
        k,
        t!("activity_title"),
        Some(t!("activity_hint", max = crate::ui::MAX_EVENTS)),
        content,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_has_its_own_icon() {
        let names = [
            level_icon(Level::Info).name,
            level_icon(Level::Good).name,
            level_icon(Level::Warn).name,
        ];
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 3, "{names:?}");
    }
}

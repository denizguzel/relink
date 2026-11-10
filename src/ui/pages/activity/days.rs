//! Which events the page shows and how they are grouped by day.

use crate::ui::{Event, Level};

/// A local calendar day: (year, month, day).
pub type Day = (u32, u32, u32);

/// Whether the filter (0 all, 1 warnings, 2 fixes) keeps an event.
pub fn keep(filter: usize, level: Level) -> bool {
    match filter {
        1 => level == Level::Warn,
        2 => level == Level::Good,
        _ => true,
    }
}

/// The events that pass `filter`, in their order (newest first).
pub fn filtered<'a>(events: impl IntoIterator<Item = &'a Event>, filter: usize) -> Vec<&'a Event> {
    events
        .into_iter()
        .filter(|e| keep(filter, e.level))
        .collect()
}

/// Splits newest-first events into one run per day.
pub fn by_day<'a>(events: &[&'a Event]) -> Vec<(Day, Vec<&'a Event>)> {
    let mut runs: Vec<(Day, Vec<&Event>)> = Vec::new();
    for &e in events {
        match runs.last_mut() {
            Some((day, run)) if *day == e.day => run.push(e),
            _ => runs.push((e.day, vec![e])),
        }
    }
    runs
}

/// How a day is titled: "today", "yesterday", or its date in the language's
/// usual order (`dd.mm.yyyy` in Turkish, `yyyy-mm-dd` otherwise).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayTitle {
    Today,
    Yesterday,
    Date(Day),
}

pub fn title(day: Day, today: Day, yesterday: Day) -> DayTitle {
    if day == today {
        DayTitle::Today
    } else if day == yesterday {
        DayTitle::Yesterday
    } else {
        DayTitle::Date(day)
    }
}

pub fn format_date((y, m, d): Day, turkish: bool) -> String {
    if turkish {
        format!("{d:02}.{m:02}.{y}")
    } else {
        format!("{y}-{m:02}-{d:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(day: Day, level: Level, text: &str) -> Event {
        Event {
            ts: 0,
            day,
            time: "12:00:00".into(),
            text: text.into(),
            level,
        }
    }

    #[test]
    fn filters_by_level() {
        let evs = [
            event((2026, 11, 10), Level::Info, "a"),
            event((2026, 11, 10), Level::Warn, "b"),
            event((2026, 11, 10), Level::Good, "c"),
        ];
        let texts = |f| {
            filtered(&evs, f)
                .iter()
                .map(|e| e.text.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(texts(0), ["a", "b", "c"]);
        assert_eq!(texts(1), ["b"]);
        assert_eq!(texts(2), ["c"]);
    }

    #[test]
    fn groups_consecutive_events_by_day() {
        let evs = [
            event((2026, 11, 10), Level::Info, "1"),
            event((2026, 11, 10), Level::Info, "2"),
            event((2026, 11, 9), Level::Info, "3"),
            event((2026, 11, 10), Level::Info, "4"),
        ];
        let refs: Vec<&Event> = evs.iter().collect();
        let runs = by_day(&refs);
        assert_eq!(
            runs.iter().map(|r| (r.0, r.1.len())).collect::<Vec<_>>(),
            [((2026, 11, 10), 2), ((2026, 11, 9), 1), ((2026, 11, 10), 1)]
        );
        assert!(by_day(&[]).is_empty());
    }

    #[test]
    fn titles_today_yesterday_or_the_date() {
        let (today, yesterday) = ((2026, 11, 10), (2026, 11, 9));
        assert_eq!(title(today, today, yesterday), DayTitle::Today);
        assert_eq!(title(yesterday, today, yesterday), DayTitle::Yesterday);
        assert_eq!(
            title((2026, 11, 5), today, yesterday),
            DayTitle::Date((2026, 11, 5))
        );
    }

    #[test]
    fn dates_follow_the_language() {
        assert_eq!(format_date((2026, 3, 7), true), "07.03.2026");
        assert_eq!(format_date((2026, 3, 7), false), "2026-03-07");
    }
}

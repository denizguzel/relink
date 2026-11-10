//! Controls used on more than one page: the interface language, the
//! notification level and the start-with-the-session switch (Settings and the
//! welcome card).

use rune::{El, Tx};
use rune_kit::{Kit, SegmentedControl, Switch};

use crate::t;
use crate::ui::Ui;
use crate::ui::theme::{THEMES, theme_index};

/// The notification levels in the order of the segmented control.
pub const LEVELS: [&str; 3] = ["all", "problems", "off"];

/// The segment of a level; an unknown value counts as "problems", the default.
pub fn level_index(level: &str) -> usize {
    LEVELS.iter().position(|&l| l == level).unwrap_or(1)
}

/// The language choices in the order of the segmented control.
pub const LANGUAGES: [&str; 3] = ["auto", "tr", "en"];

/// The segment of a language setting; an unknown value counts as "auto".
pub fn language_index(language: &str) -> usize {
    LANGUAGES.iter().position(|&l| l == language).unwrap_or(0)
}

/// The interface language: follow the system, Turkish or English. The
/// languages are named in themselves, so they stay readable whatever the
/// current language is.
pub fn language(s: &Ui, k: &Kit) -> El {
    SegmentedControl::new(
        &[t!("lang_auto"), t!("lang_tr"), t!("lang_en")],
        Some(language_index(&s.cfg.language)),
    )
    .label(t!("settings_language"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| s.change(|c| c.language = LANGUAGES[i].into()))
    .build(k)
}

/// The appearance: follow the system, light or dark.
pub fn theme(s: &Ui, k: &Kit) -> El {
    SegmentedControl::new(
        &[t!("theme_system"), t!("theme_light"), t!("theme_dark")],
        Some(theme_index(&s.cfg.theme)),
    )
    .label(t!("theme"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| s.change(|c| c.theme = THEMES[i].into()))
    .build(k)
}

pub fn notifications(s: &Ui, k: &Kit) -> El {
    SegmentedControl::new(
        &[t!("notify_all"), t!("notify_problems"), t!("notify_off")],
        Some(level_index(&s.cfg.notifications)),
    )
    .label(t!("notifications"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| s.change(|c| c.notifications = LEVELS[i].into()))
    .build(k)
}

pub fn startup(s: &Ui, k: &Kit) -> El {
    Switch::new(s.startup)
        .label(t!("startup"))
        .hide_label()
        .on_toggle(|s: &mut Ui| {
            if let Some(l) = &s.link {
                let _ = crate::platform::set_startup(&l.exe, !s.startup);
                s.startup = crate::platform::startup_enabled(&l.exe);
            }
        })
        .build(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_map_to_segments() {
        assert_eq!(level_index("all"), 0);
        assert_eq!(level_index("problems"), 1);
        assert_eq!(level_index("off"), 2);
        assert_eq!(level_index("garbage"), 1);
    }

    #[test]
    fn languages_map_to_segments() {
        assert_eq!(language_index("auto"), 0);
        assert_eq!(language_index("tr"), 1);
        assert_eq!(language_index("en"), 2);
        assert_eq!(language_index("de"), 0, "unknown follows the system");
    }
}

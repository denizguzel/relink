//! Controls used on more than one page: the interface language, the
//! notification level and the start-with-the-session switch (Settings and the
//! welcome card), and the Settings' small segmented choices.

use rune::{El, Tx, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl, Switch};

use crate::t;
use crate::ui::Ui;
use crate::ui::icons;
use crate::ui::parts::setting_row;
use crate::ui::theme::{THEMES, theme_index};
use crate::ui::zoom;

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

/// What happens when the user is back and the VPN is gone, in the order of the
/// segmented control. Without FortiClient there is nothing to open.
pub fn unlock_modes(client: bool) -> &'static [&'static str] {
    if client {
        &["notify", "open", "off"]
    } else {
        &["notify", "off"]
    }
}

/// The segment of an unlock setting; an unknown value (or "open" with nothing
/// to open, which then notifies) counts as "notify".
pub fn unlock_index(mode: &str, client: bool) -> usize {
    unlock_modes(client)
        .iter()
        .position(|&m| m == mode)
        .unwrap_or(0)
}

fn vpn_after_unlock(s: &Ui, k: &Kit) -> El {
    let labels = if s.vpn_client {
        vec![t!("unlock_notify"), t!("unlock_open"), t!("unlock_off")]
    } else {
        vec![t!("unlock_notify"), t!("unlock_off")]
    };
    SegmentedControl::new(
        &labels,
        Some(unlock_index(&s.cfg.vpn_after_unlock, s.vpn_client)),
    )
    .label(t!("vpn_after_unlock"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| {
        let mode = unlock_modes(s.vpn_client)[i];
        s.change(|c| c.vpn_after_unlock = mode.into())
    })
    .build(k)
}

/// The "after lock or sleep" row (Settings and the welcome card): where
/// Re-Link sees the lock (Windows with FortiClient, Linux).
pub fn unlock_row(key: &str, s: &Ui, k: &Kit) -> Option<El> {
    if !s.vpn_client && !cfg!(target_os = "linux") {
        return None;
    }
    Some(setting_row(
        key,
        k,
        &icons::LOCK_OPEN,
        t!("vpn_after_unlock"),
        Some(if s.vpn_client {
            t!("vpn_after_unlock_desc")
        } else {
            t!("vpn_after_unlock_desc_no_client")
        }),
        vpn_after_unlock(s, k),
    ))
}

/// The "warn before the session ends" row (Settings and the welcome card).
pub fn session_row(key: &str, s: &Ui, k: &Kit) -> El {
    let switch = Switch::new(s.cfg.session_warning)
        .label(t!("session_warning"))
        .hide_label()
        .on_toggle(|s: &mut Ui| s.change(|c| c.session_warning = !c.session_warning))
        .build(k);
    setting_row(
        key,
        k,
        &icons::CLOCK,
        t!("session_warning"),
        Some(t!("session_warning_desc")),
        switch,
    )
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

/// Zoom: smaller, the level (a press on it resets), larger.
pub fn zoom(s: &Ui, k: &Kit) -> El {
    let level = s.cfg.zoom_percent.clamp(zoom::MIN, zoom::MAX);
    let step = |key: &'static str, label: &'static str, up: bool, enabled: bool| {
        Button::new(label)
            .variant(ButtonVariant::Neutral)
            .disabled(!enabled)
            .on_press(move |s: &mut Ui| {
                let next = zoom::step(s.cfg.zoom_percent, up);
                s.change(|c| c.zoom_percent = next);
            })
            .build(k)
            .key(key)
    };
    let reset = Button::new(t!("zoom_reset"))
        .variant(ButtonVariant::Ghost)
        .disabled(level == 100)
        .on_press(|s: &mut Ui| s.change(|c| c.zoom_percent = 100))
        .build(k)
        .key("zoom-reset");
    row()
        .key("zoom-controls")
        .gap(8.0)
        .center_y()
        .child(step("zoom-out", "\u{2212}", false, level > zoom::MIN))
        .child(
            el().w(48.0).center().child(
                text(format!("{level}%"))
                    .font(k.ty.ui_medium)
                    .color(k.c.text_base),
            ),
        )
        .child(step("zoom-in", "+", true, level < zoom::MAX))
        .child(reset)
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

    #[test]
    fn unlock_modes_map_to_segments() {
        assert_eq!(unlock_index("notify", true), 0);
        assert_eq!(unlock_index("open", true), 1);
        assert_eq!(unlock_index("off", true), 2);
        assert_eq!(unlock_index("", true), 0, "unknown notifies, the default");
        assert_eq!(unlock_index("off", false), 1);
        assert_eq!(unlock_index("open", false), 0, "nothing to open: notifies");
    }
}

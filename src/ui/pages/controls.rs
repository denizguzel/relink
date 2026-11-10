//! Controls used on more than one page: the notification level and the
//! start-with-the-session switch (Settings and the welcome card).

use rune::{El, Tx};
use rune_kit::{Kit, SegmentedControl, Switch};

use crate::t;
use crate::ui::Ui;

/// The notification levels in the order of the segmented control.
pub const LEVELS: [&str; 3] = ["all", "problems", "off"];

/// The segment of a level; an unknown value counts as "problems", the default.
pub fn level_index(level: &str) -> usize {
    LEVELS.iter().position(|&l| l == level).unwrap_or(1)
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
}

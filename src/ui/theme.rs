//! The window's light or dark appearance: the setting and how it resolves.

/// The choices in the order of the segmented control.
pub const THEMES: [&str; 3] = ["system", "light", "dark"];

/// The segment of a theme setting; an unknown value counts as "system".
pub fn theme_index(theme: &str) -> usize {
    THEMES.iter().position(|&t| t == theme).unwrap_or(0)
}

/// Whether the window draws dark: the setting, or the system's appearance
/// when it follows the system. Resolved for each view, so "system" keeps
/// following the operating system live.
pub fn is_dark(theme: &str, system_dark: bool) -> bool {
    match theme {
        "light" => false,
        "dark" => true,
        _ => system_dark,
    }
}

/// What the title bar must be painted as: `Some(dark)` when the setting forces
/// an appearance different from the system's (the system draws the title bar
/// itself, so only then is it wrong), `None` when it already matches.
pub fn forced_title_bar(theme: &str, system_dark: bool) -> Option<bool> {
    let dark = is_dark(theme, system_dark);
    (dark != system_dark).then_some(dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_overrides_the_system_and_system_follows_it() {
        for system in [false, true] {
            assert!(!is_dark("light", system));
            assert!(is_dark("dark", system));
            assert_eq!(is_dark("system", system), system);
            assert_eq!(is_dark("nonsense", system), system);
        }
    }

    #[test]
    fn settings_map_to_segments() {
        assert_eq!(theme_index("system"), 0);
        assert_eq!(theme_index("light"), 1);
        assert_eq!(theme_index("dark"), 2);
        assert_eq!(theme_index("x"), 0);
    }

    #[test]
    fn the_title_bar_is_only_forced_when_it_would_otherwise_be_wrong() {
        assert_eq!(forced_title_bar("system", true), None);
        assert_eq!(forced_title_bar("dark", true), None, "already dark");
        assert_eq!(forced_title_bar("light", false), None, "already light");
        assert_eq!(forced_title_bar("dark", false), Some(true));
        assert_eq!(forced_title_bar("light", true), Some(false));
    }
}

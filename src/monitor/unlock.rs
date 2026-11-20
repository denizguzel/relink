//! Noticing that the VPN dropped while the user was away (the session locked,
//! or the computer asleep), so Re-Link can offer to reconnect the moment they
//! are back. Pure: fed the lock state, whether the computer just woke, and
//! whether the VPN is up after each check.

/// What to do when the user is back and the VPN is gone (`vpnAfterUnlock`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnlockMode {
    /// Show a notification; clicking it opens FortiClient.
    Notify,
    /// Open FortiClient on its Remote Access page right away.
    Open,
    Off,
}

impl UnlockMode {
    /// The setting's value; anything unknown counts as the default, "notify".
    pub fn parse(value: &str) -> UnlockMode {
        match value {
            "open" => UnlockMode::Open,
            "off" => UnlockMode::Off,
            _ => UnlockMode::Notify,
        }
    }
}

/// Whether the computer slept between two checks: the wall clock moved on far
/// more than the monitor waited (its own timer stands still in sleep).
/// `prev`/`now`: Unix seconds of the two checks; `interval`: the longest wait.
pub fn slept(prev: Option<u64>, now: u64, interval: std::time::Duration) -> bool {
    prev.is_some_and(|p| now.saturating_sub(p) > interval.as_secs() + 60)
}

/// The away state carried between checks.
#[derive(Debug, Default)]
pub struct UnlockWatch {
    away: bool,
    /// The VPN at the last check while the user was there.
    vpn_up: bool,
    /// The VPN when they left (as of the last check before).
    vpn_at_leave: bool,
}

impl UnlockWatch {
    /// Takes one check: the lock state (`None`: unknown, counts as unlocked),
    /// whether the computer has just woken, and the VPN. Returns true once, at
    /// the first check with the user back, when the VPN was up before they left
    /// and is down now.
    pub fn observe(&mut self, locked: Option<bool>, woke: bool, vpn_up: bool) -> bool {
        if (woke || locked == Some(true)) && !self.away {
            self.away = true;
            self.vpn_at_leave = self.vpn_up;
        }
        if locked == Some(true) {
            return false;
        }
        let lost = self.away && self.vpn_at_leave && !vpn_up;
        self.away = false;
        self.vpn_up = vpn_up;
        lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_vpn_lost_during_the_lock_is_reported_once_at_unlock() {
        let mut w = UnlockWatch::default();
        assert!(!w.observe(Some(false), false, true));
        assert!(!w.observe(Some(true), false, true), "locked, still up");
        assert!(!w.observe(Some(true), false, false), "dropped while locked");
        assert!(w.observe(Some(false), false, false), "back: VPN is gone");
        assert!(!w.observe(Some(false), false, false), "only once");
    }

    #[test]
    fn a_lock_and_drop_between_two_checks_still_counts() {
        let mut w = UnlockWatch::default();
        w.observe(Some(false), false, true);
        assert!(!w.observe(Some(true), false, false));
        assert!(w.observe(Some(false), false, false));
    }

    #[test]
    fn nothing_when_the_vpn_survived_or_came_back() {
        let mut w = UnlockWatch::default();
        w.observe(Some(false), false, true);
        w.observe(Some(true), false, true);
        assert!(!w.observe(Some(false), false, true));
        w.observe(Some(true), false, false);
        assert!(
            !w.observe(Some(false), false, true),
            "reconnected before unlock"
        );
    }

    #[test]
    fn nothing_when_the_vpn_was_already_down_before_the_lock() {
        let mut w = UnlockWatch::default();
        w.observe(Some(false), false, false);
        w.observe(Some(true), false, false);
        assert!(!w.observe(Some(false), false, false));
    }

    #[test]
    fn a_start_while_locked_or_an_unknown_state_says_nothing() {
        let mut w = UnlockWatch::default();
        w.observe(Some(true), false, true);
        assert!(
            !w.observe(Some(false), false, false),
            "no check before the lock"
        );
        let mut w = UnlockWatch::default();
        w.observe(Some(false), false, true);
        assert!(!w.observe(None, false, false), "unknown is not a lock");
    }

    #[test]
    fn waking_without_a_lock_counts_as_being_back() {
        let mut w = UnlockWatch::default();
        w.observe(None, false, true);
        assert!(w.observe(None, true, false), "slept, VPN gone");
        let mut w = UnlockWatch::default();
        w.observe(Some(false), false, true);
        assert!(!w.observe(Some(false), true, true), "slept, VPN still up");
    }

    #[test]
    fn waking_to_the_lock_screen_waits_for_the_unlock() {
        let mut w = UnlockWatch::default();
        w.observe(Some(false), false, true);
        assert!(!w.observe(Some(true), true, false), "woke, still locked");
        assert!(w.observe(Some(false), false, false), "unlocked");
    }

    #[test]
    fn a_long_gap_between_checks_means_sleep() {
        let i = Duration::from_secs(15);
        assert!(!slept(None, 1000, i));
        assert!(!slept(Some(1000), 1015, i));
        assert!(!slept(Some(1000), 1070, i), "a slow check is not sleep");
        assert!(slept(Some(1000), 1000 + 15 + 61, i));
    }

    #[test]
    fn unknown_settings_mean_notify() {
        assert_eq!(UnlockMode::parse("open"), UnlockMode::Open);
        assert_eq!(UnlockMode::parse("off"), UnlockMode::Off);
        assert_eq!(UnlockMode::parse(""), UnlockMode::Notify);
        assert_eq!(UnlockMode::parse("notify"), UnlockMode::Notify);
    }
}

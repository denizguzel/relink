//! Pausing notifications: how long a pause lasts and how much is left.

/// The length of the pause the buttons start: one hour.
pub const PAUSE_SECS: u64 = 3600;

/// Whole minutes left of a pause that ends at `until` (rounded up, at least
/// one), or None when no pause is running at `now`.
pub fn minutes_left(until: Option<u64>, now: u64) -> Option<u64> {
    let left = until?.checked_sub(now).filter(|&s| s > 0)?;
    Some(left.div_ceil(60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_pause_means_nothing_left() {
        assert_eq!(minutes_left(None, 1000), None);
    }

    #[test]
    fn an_expired_pause_has_nothing_left() {
        assert_eq!(minutes_left(Some(1000), 1000), None);
        assert_eq!(minutes_left(Some(1000), 5000), None);
    }

    #[test]
    fn minutes_round_up_and_never_show_zero() {
        assert_eq!(minutes_left(Some(1000 + 3600), 1000), Some(60));
        assert_eq!(minutes_left(Some(1000 + 61), 1000), Some(2));
        assert_eq!(minutes_left(Some(1000 + 60), 1000), Some(1));
        assert_eq!(minutes_left(Some(1000 + 1), 1000), Some(1));
    }

    #[test]
    fn one_hour_is_the_default_pause() {
        assert_eq!(PAUSE_SECS, 3600);
    }
}

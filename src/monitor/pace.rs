//! How soon the next check runs while the VPN is down or coming up. Pure, so
//! the policy is tested.

use std::time::Duration;

/// While the VPN is down, look every second at first (it is often being
/// reconnected), then calm down: hours at home without the VPN need no
/// second-by-second checks.
pub const FAST_AFTER_DROP: Duration = Duration::from_secs(120);
/// The pace once the VPN has stayed down for a while.
pub const DOWN_PACE: Duration = Duration::from_secs(5);

/// The longest wait before the next check, or `None` when the configured
/// interval applies (VPN up and settled). `down_for`: how long the VPN has
/// been down (None: it is up, or there is no report yet).
pub fn cap(
    have_report: bool,
    vpn_up: bool,
    settling: bool,
    down_for: Option<Duration>,
) -> Option<Duration> {
    if !have_report || settling {
        return Some(Duration::from_secs(1));
    }
    if vpn_up {
        return None;
    }
    match down_for {
        Some(d) if d >= FAST_AFTER_DROP => Some(DOWN_PACE),
        _ => Some(Duration::from_secs(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn a_settled_vpn_uses_the_interval() {
        assert_eq!(cap(true, true, false, None), None);
    }

    #[test]
    fn the_start_and_arriving_routes_are_watched_every_second() {
        assert_eq!(cap(false, false, false, None), Some(S(1)));
        assert_eq!(cap(true, true, true, None), Some(S(1)));
    }

    #[test]
    fn a_dropped_vpn_is_watched_closely_then_calmly() {
        assert_eq!(cap(true, false, false, Some(S(0))), Some(S(1)));
        assert_eq!(cap(true, false, false, Some(S(119))), Some(S(1)));
        assert_eq!(cap(true, false, false, Some(S(120))), Some(DOWN_PACE));
        assert_eq!(cap(true, false, false, Some(S(36_000))), Some(DOWN_PACE));
    }
}

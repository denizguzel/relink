//! How long the VPN has been connected, and when the session will likely end.
//! SSL VPN gateways often end a session after a fixed time (8 or 12 hours, say);
//! Re-Link learns that time from the sessions it saw end, and warns a little
//! before it. Pure: the times are passed in, the history lives in
//! `relink-state.json`.

/// Shorter sessions say nothing about a limit (a sleep, a manual disconnect).
pub const MIN_SESSION: u64 = 3600;
/// Sessions ending this close to each other count as ending at the same limit.
pub const TOLERANCE: u64 = 5 * 60;
/// How many sessions must end at the same time before it counts as a limit.
pub const CLUSTER: usize = 3;
/// How many sessions are remembered.
pub const KEEP: usize = 20;
/// How long before the likely end the warning comes.
pub const WARN_BEFORE: u64 = 10 * 60;
/// A start older than this is not trusted after a restart (the VPN has
/// reconnected meanwhile, unseen).
pub const STALE_START: u64 = 24 * 3600;

/// The session limit the history suggests: the shortest duration of the
/// largest group of long sessions that ended within `TOLERANCE` of each other,
/// when that group has at least `CLUSTER` members.
pub fn limit(durations: &[u64]) -> Option<u64> {
    let long: Vec<u64> = durations
        .iter()
        .copied()
        .filter(|&d| d >= MIN_SESSION)
        .collect();
    long.iter()
        .filter_map(|&d| {
            let near: Vec<u64> = long
                .iter()
                .copied()
                .filter(|&e| e >= d && e - d <= TOLERANCE)
                .collect();
            (near.len() >= CLUSTER).then_some((near.len(), d))
        })
        // The largest group; on a tie, the earlier end (warn too early rather than late).
        .max_by_key(|&(n, d)| (n, std::cmp::Reverse(d)))
        .map(|(_, d)| d)
}

/// What the latest check means for the session record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub vpn_up: bool,
    pub now: u64,
    /// The end of a session is known to the second: Re-Link was watching (not
    /// its first check, not just back from sleep).
    pub watched: bool,
}

/// Follows the VPN going up and down. Returns whether the record changed.
pub fn follow(since: &mut Option<u64>, sessions: &mut Vec<u64>, seen: Seen) -> bool {
    match (*since, seen.vpn_up) {
        (None, true) => {
            *since = Some(seen.now);
            true
        }
        // A start remembered from long ago (Re-Link was off): unknown, restart it.
        (Some(start), true) if !seen.watched && seen.now.saturating_sub(start) > STALE_START => {
            *since = Some(seen.now);
            true
        }
        (Some(start), false) => {
            *since = None;
            if seen.watched {
                sessions.push(seen.now.saturating_sub(start));
                let extra = sessions.len().saturating_sub(KEEP);
                sessions.drain(..extra);
            }
            true
        }
        _ => false,
    }
}

/// Whether to warn now: within `WARN_BEFORE` of the limit, before it.
pub fn warn_due(since: Option<u64>, limit: Option<u64>, now: u64) -> bool {
    let (Some(since), Some(limit)) = (since, limit) else {
        return false;
    };
    let up = now.saturating_sub(since);
    up + WARN_BEFORE >= limit && up < limit
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: u64 = 3600;

    #[test]
    fn three_sessions_ending_together_reveal_the_limit() {
        assert_eq!(limit(&[]), None);
        assert_eq!(limit(&[8 * H, 8 * H + 60]), None, "two are a coincidence");
        assert_eq!(
            limit(&[8 * H + 60, 600, 8 * H, 3 * H, 8 * H + 120]),
            Some(8 * H)
        );
    }

    #[test]
    fn short_sessions_are_ignored_and_the_largest_group_wins() {
        assert_eq!(limit(&[1200, 1200, 1250, 1300]), None);
        let d = [12 * H, 12 * H, 12 * H, 12 * H, 8 * H, 8 * H, 8 * H];
        assert_eq!(limit(&d), Some(12 * H));
        let tie = [12 * H, 12 * H, 12 * H, 8 * H, 8 * H, 8 * H];
        assert_eq!(limit(&tie), Some(8 * H), "on a tie, the earlier end");
    }

    fn seen(vpn_up: bool, now: u64) -> Seen {
        Seen {
            vpn_up,
            now,
            watched: true,
        }
    }

    #[test]
    fn a_watched_session_is_recorded_when_it_ends() {
        let (mut since, mut s) = (None, vec![]);
        assert!(follow(&mut since, &mut s, seen(true, 100)));
        assert!(!follow(&mut since, &mut s, seen(true, 200)));
        assert!(follow(&mut since, &mut s, seen(false, 100 + 8 * H)));
        assert_eq!((since, s), (None, vec![8 * H]));
    }

    #[test]
    fn an_end_nobody_saw_is_not_recorded() {
        let (mut since, mut s) = (Some(100), vec![]);
        let away = Seen {
            vpn_up: false,
            now: 100 + 9 * H,
            watched: false,
        };
        assert!(follow(&mut since, &mut s, away));
        assert_eq!((since, s.len()), (None, 0));
    }

    #[test]
    fn a_start_from_long_ago_is_not_trusted_after_a_restart() {
        let (mut since, mut s) = (Some(100), vec![]);
        let now = 100 + 2 * STALE_START;
        let first = Seen {
            vpn_up: true,
            now,
            watched: false,
        };
        assert!(follow(&mut since, &mut s, first));
        assert_eq!(since, Some(now));
        // A recent one is kept (Re-Link was restarted by an update, say).
        let mut since = Some(now - 600);
        assert!(!follow(&mut since, &mut s, first));
        assert_eq!(since, Some(now - 600));
    }

    #[test]
    fn only_the_latest_sessions_are_kept() {
        let (mut since, mut s) = (None, (0..KEEP as u64).collect::<Vec<_>>());
        follow(&mut since, &mut s, seen(true, 0));
        follow(&mut since, &mut s, seen(false, 5 * H));
        assert_eq!(s.len(), KEEP);
        assert_eq!(*s.last().unwrap(), 5 * H);
        assert_eq!(s[0], 1, "the oldest went");
    }

    #[test]
    fn the_warning_comes_in_the_last_minutes() {
        let lim = Some(8 * H);
        assert!(!warn_due(Some(0), lim, 7 * H));
        assert!(warn_due(Some(0), lim, 8 * H - WARN_BEFORE));
        assert!(warn_due(Some(0), lim, 8 * H - 1));
        assert!(!warn_due(Some(0), lim, 8 * H), "past the limit");
        assert!(!warn_due(None, lim, 8 * H - 60));
        assert!(!warn_due(Some(0), None, 8 * H - 60));
    }
}

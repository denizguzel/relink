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
/// After a restart, a start is trusted only if the VPN was seen up this
/// recently; a longer gap (computer off, Re-Link closed) means it reconnected
/// meanwhile, unseen.
pub const STALE_GAP: u64 = 15 * 60;
/// How often the "last seen up" time is refreshed on disk.
pub const SEEN_EVERY: u64 = 5 * 60;

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
pub fn follow(
    since: &mut Option<u64>,
    last_up: &mut Option<u64>,
    sessions: &mut Vec<u64>,
    seen: Seen,
) -> bool {
    let refresh = seen.vpn_up && last_up.is_none_or(|t| seen.now.saturating_sub(t) >= SEEN_EVERY);
    let changed = match (*since, seen.vpn_up) {
        (None, true) => {
            *since = Some(seen.now);
            true
        }
        // Re-Link was not watching and the VPN was last seen up long ago: it
        // reconnected unseen, so the remembered start is wrong. Restart it.
        (Some(_), true)
            if !seen.watched && last_up.is_none_or(|t| seen.now.saturating_sub(t) > STALE_GAP) =>
        {
            *since = Some(seen.now);
            true
        }
        (Some(start), false) => {
            *since = None;
            *last_up = None;
            if seen.watched {
                sessions.push(seen.now.saturating_sub(start));
                let extra = sessions.len().saturating_sub(KEEP);
                sessions.drain(..extra);
            }
            true
        }
        _ => false,
    };
    if refresh {
        *last_up = Some(seen.now);
    }
    changed || refresh
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

    fn unwatched(vpn_up: bool, now: u64) -> Seen {
        Seen {
            vpn_up,
            now,
            watched: false,
        }
    }

    #[test]
    fn a_watched_session_is_recorded_when_it_ends() {
        let (mut since, mut last, mut s) = (None, None, vec![]);
        assert!(follow(&mut since, &mut last, &mut s, seen(true, 100)));
        assert!(!follow(&mut since, &mut last, &mut s, seen(true, 200)));
        assert!(follow(
            &mut since,
            &mut last,
            &mut s,
            seen(false, 100 + 8 * H)
        ));
        assert_eq!((since, last, s), (None, None, vec![8 * H]));
    }

    #[test]
    fn an_end_nobody_saw_is_not_recorded() {
        let (mut since, mut last, mut s) = (Some(100), Some(100), vec![]);
        let away = unwatched(false, 100 + 9 * H);
        assert!(follow(&mut since, &mut last, &mut s, away));
        assert_eq!((since, s.len()), (None, 0));
    }

    #[test]
    fn a_night_with_the_computer_off_restarts_the_start() {
        // Seen up at 18:00, back 20 hours later: the VPN reconnected unseen.
        let (mut since, mut last, mut s) = (Some(100), Some(100 + 8 * H), vec![]);
        let now = 100 + 28 * H;
        assert!(follow(&mut since, &mut last, &mut s, unwatched(true, now)));
        assert_eq!((since, last), (Some(now), Some(now)));
    }

    #[test]
    fn an_old_state_without_last_seen_is_not_trusted() {
        let (mut since, mut last, mut s) = (Some(100), None, vec![]);
        assert!(follow(&mut since, &mut last, &mut s, unwatched(true, 600)));
        assert_eq!(since, Some(600));
    }

    #[test]
    fn a_quick_restart_keeps_the_start() {
        // Re-Link restarted by an update: seen up a minute ago.
        let (mut since, mut last, mut s) = (Some(100), Some(1000), vec![]);
        assert!(!follow(
            &mut since,
            &mut last,
            &mut s,
            unwatched(true, 1060)
        ));
        assert_eq!((since, last), (Some(100), Some(1000)));
    }

    #[test]
    fn last_seen_is_refreshed_every_few_minutes() {
        let (mut since, mut last, mut s) = (Some(0), Some(1000), vec![]);
        assert!(!follow(
            &mut since,
            &mut last,
            &mut s,
            seen(true, 1000 + SEEN_EVERY - 1)
        ));
        assert!(follow(
            &mut since,
            &mut last,
            &mut s,
            seen(true, 1000 + SEEN_EVERY)
        ));
        assert_eq!(last, Some(1000 + SEEN_EVERY));
    }

    #[test]
    fn only_the_latest_sessions_are_kept() {
        let (mut since, mut last, mut s) = (None, None, (0..KEEP as u64).collect::<Vec<_>>());
        follow(&mut since, &mut last, &mut s, seen(true, 0));
        follow(&mut since, &mut last, &mut s, seen(false, 5 * H));
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

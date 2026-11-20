//! The last name resolution. While the VPN is down or its routes arrive,
//! checks run every second, but looking up 50 names that often is pointless.
//! While the routes arrive the names are looked up more often than while the
//! VPN is down: DNS switches to the company's servers then and names move to
//! other addresses, which should show up early in the wait, not after it.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

/// How often names are looked up while the VPN is down.
const DOWN_REFRESH: Duration = Duration::from_secs(10);
/// How often names are looked up while the VPN's routes arrive.
const SETTLING_REFRESH: Duration = Duration::from_secs(3);

/// How old the last answer may be before names are looked up again (None:
/// at every check, as when the VPN is up and settled).
pub(super) fn refresh(vpn_up: bool, settling: bool) -> Option<Duration> {
    if !vpn_up {
        Some(DOWN_REFRESH)
    } else if settling {
        Some(SETTLING_REFRESH)
    } else {
        None
    }
}

#[derive(Default)]
pub(super) struct NameCache {
    last: Option<Instant>,
    hosts: BTreeMap<String, Vec<Ipv4Addr>>,
}

impl NameCache {
    /// Whether to resolve now, given the [`refresh`] interval.
    pub(super) fn due(&self, refresh: Option<Duration>) -> bool {
        self.due_at(refresh, Instant::now())
    }

    fn due_at(&self, refresh: Option<Duration>, now: Instant) -> bool {
        refresh.is_none_or(|every| self.last.is_none_or(|t| now.duration_since(t) >= every))
    }

    pub(super) fn store(&mut self, hosts: &BTreeMap<String, Vec<Ipv4Addr>>) {
        self.store_at(hosts, Instant::now());
    }

    fn store_at(&mut self, hosts: &BTreeMap<String, Vec<Ipv4Addr>>, now: Instant) {
        self.last = Some(now);
        self.hosts = hosts.clone();
    }

    pub(super) fn hosts(&self) -> BTreeMap<String, Vec<Ipv4Addr>> {
        self.hosts.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_always_resolves() {
        let cache = NameCache::default();
        assert!(cache.due_at(None, Instant::now()));
    }

    #[test]
    fn names_are_looked_up_more_often_while_the_routes_arrive_than_while_down() {
        assert_eq!(refresh(false, false), Some(DOWN_REFRESH));
        assert_eq!(refresh(false, true), Some(DOWN_REFRESH));
        assert_eq!(refresh(true, true), Some(SETTLING_REFRESH));
        assert_eq!(refresh(true, false), None);
        assert!(SETTLING_REFRESH < DOWN_REFRESH);
    }

    #[test]
    fn a_refresh_interval_resolves_first_then_only_when_the_answer_is_that_old() {
        let (t0, mut cache) = (Instant::now(), NameCache::default());
        let every = Some(DOWN_REFRESH);
        assert!(cache.due_at(every, t0), "nothing cached yet");
        cache.store_at(
            &BTreeMap::from([("a.example".to_string(), vec![Ipv4Addr::new(1, 1, 1, 1)])]),
            t0,
        );
        assert!(!cache.due_at(every, t0 + Duration::from_secs(3)));
        assert!(!cache.due_at(every, t0 + Duration::from_secs(9)));
        assert!(cache.due_at(every, t0 + Duration::from_secs(10)));
        assert!(cache.due_at(Some(SETTLING_REFRESH), t0 + Duration::from_secs(3)));
        assert!(
            cache.due_at(None, t0 + Duration::from_secs(1)),
            "normal mode ignores the cache age"
        );
    }

    #[test]
    fn the_cached_answer_is_returned_as_stored() {
        let mut cache = NameCache::default();
        let hosts = BTreeMap::from([("b.example".to_string(), vec![Ipv4Addr::new(2, 2, 2, 2)])]);
        cache.store_at(&hosts, Instant::now());
        assert_eq!(cache.hosts(), hosts);
    }
}

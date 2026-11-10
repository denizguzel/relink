//! The last name resolution. While the VPN is down or its routes arrive,
//! checks run every second, but looking up 50 names that often is pointless.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

const FAST_REFRESH: Duration = Duration::from_secs(10);

#[derive(Default)]
pub(super) struct NameCache {
    last: Option<Instant>,
    hosts: BTreeMap<String, Vec<Ipv4Addr>>,
}

impl NameCache {
    /// Whether to resolve now: always in normal mode, rarely in fast mode.
    pub(super) fn due(&self, fast: bool) -> bool {
        self.due_at(fast, Instant::now())
    }

    fn due_at(&self, fast: bool, now: Instant) -> bool {
        !fast
            || self
                .last
                .is_none_or(|t| now.duration_since(t) >= FAST_REFRESH)
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
        assert!(cache.due_at(false, Instant::now()));
    }

    #[test]
    fn fast_mode_resolves_first_then_only_every_ten_seconds() {
        let (t0, mut cache) = (Instant::now(), NameCache::default());
        assert!(cache.due_at(true, t0), "nothing cached yet");
        cache.store_at(
            &BTreeMap::from([("a.example".to_string(), vec![Ipv4Addr::new(1, 1, 1, 1)])]),
            t0,
        );
        assert!(!cache.due_at(true, t0 + Duration::from_secs(3)));
        assert!(!cache.due_at(true, t0 + Duration::from_secs(9)));
        assert!(cache.due_at(true, t0 + Duration::from_secs(10)));
        assert!(
            cache.due_at(false, t0 + Duration::from_secs(1)),
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

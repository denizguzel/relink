//! The pause: notifications and automatic fixes are muted until a deadline.

use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Pause {
    until: Option<Instant>,
}

impl Pause {
    pub(super) fn start(&mut self, secs: u64, now: Instant) {
        self.until = Some(now + Duration::from_secs(secs));
    }

    pub(super) fn clear(&mut self) {
        self.until = None;
    }

    /// Whether a pause is running at `now`.
    pub(super) fn active(&self, now: Instant) -> bool {
        self.until.is_some_and(|t| now < t)
    }

    /// Ends an elapsed pause; whether one just ended.
    pub(super) fn expire(&mut self, now: Instant) -> bool {
        if self.until.is_some() && !self.active(now) {
            self.until = None;
            return true;
        }
        false
    }

    /// The end of the pause as unix seconds, for the window.
    pub(super) fn until_unix(&self, now: Instant, unix_now: u64) -> Option<u64> {
        let until = self.until.filter(|&t| now < t)?;
        Some(unix_now + until.duration_since(now).as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pause_runs_until_its_deadline() {
        let (t0, mut p) = (Instant::now(), Pause::default());
        assert!(!p.active(t0));
        p.start(60, t0);
        assert!(p.active(t0 + Duration::from_secs(59)));
        assert!(!p.active(t0 + Duration::from_secs(60)));
    }

    #[test]
    fn expiring_reports_once() {
        let (t0, mut p) = (Instant::now(), Pause::default());
        assert!(!p.expire(t0), "nothing to expire");
        p.start(10, t0);
        assert!(!p.expire(t0 + Duration::from_secs(5)));
        assert!(p.expire(t0 + Duration::from_secs(11)));
        assert!(!p.expire(t0 + Duration::from_secs(12)), "already ended");
    }

    #[test]
    fn clearing_ends_it_early() {
        let (t0, mut p) = (Instant::now(), Pause::default());
        p.start(3600, t0);
        p.clear();
        assert!(!p.active(t0));
        assert_eq!(p.until_unix(t0, 1000), None);
    }

    #[test]
    fn the_window_gets_the_deadline_in_unix_time() {
        let (t0, mut p) = (Instant::now(), Pause::default());
        p.start(3600, t0);
        assert_eq!(
            p.until_unix(t0 + Duration::from_secs(600), 5000),
            Some(5000 + 3000)
        );
    }
}

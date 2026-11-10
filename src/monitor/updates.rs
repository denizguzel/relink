//! The periodic "is there a newer release?" check. The network call runs on
//! its own short-lived thread so a slow connection never delays a check of
//! the VPN; the answer reaches the window through its handle.

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use super::Monitor;
use super::notify::notify;
use crate::t;
use crate::ui::Level;
use crate::update;

/// The first look, a little after start so launching stays quiet.
const FIRST: Duration = Duration::from_secs(30);
/// How often to look once a check worked.
const EVERY: Duration = Duration::from_secs(12 * 60 * 60);
/// How soon to try again after a failed check (offline, proxy, rate limit).
const RETRY: Duration = Duration::from_secs(60 * 60);

/// When the next check is due.
pub(super) struct Schedule {
    next: Instant,
}

impl Schedule {
    pub(super) fn new(now: Instant) -> Schedule {
        Schedule { next: now + FIRST }
    }

    pub(super) fn due(&self, now: Instant) -> bool {
        now >= self.next
    }

    /// Plans the following check from the result of this one.
    pub(super) fn done(&mut self, now: Instant, worked: bool) {
        self.next = now + if worked { EVERY } else { RETRY };
    }
}

type Answer = Result<Option<update::Available>, String>;

/// The check in flight, if any, and when the next one is due.
pub(super) struct Updates {
    schedule: Schedule,
    running: Option<Receiver<Answer>>,
    /// The release the check found, for "update and restart".
    found: Option<update::Available>,
}

impl Updates {
    pub(super) fn new(now: Instant) -> Updates {
        Updates {
            schedule: Schedule::new(now),
            running: None,
            found: None,
        }
    }
}

impl Monitor {
    /// Collects a finished check, then starts the next one when it is due and
    /// the setting allows it. Never blocks.
    pub(super) fn maybe_check_updates(&mut self) {
        let now = Instant::now();
        if let Some(rx) = &self.updates.running {
            match rx.try_recv() {
                Ok(answer) => {
                    self.updates.running = None;
                    self.updates.schedule.done(now, answer.is_ok());
                    if let Ok(Some(found)) = answer {
                        self.found_update(found);
                    }
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.updates.running = None;
                    self.updates.schedule.done(now, false);
                }
            }
            return;
        }
        if !self.updates.schedule.due(now) {
            return;
        }
        if !self.cfg.lock().unwrap().check_updates {
            self.updates.schedule.done(now, true);
            return;
        }
        let (tx, rx) = channel();
        self.updates.running = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(update::check(crate::VERSION));
        });
    }

    fn found_update(&mut self, found: update::Available) {
        let text = t!("ev_update_available", version = found.version);
        notify(
            "update",
            &t!("update_available", version = found.version),
            &t!("update_available_body"),
            None,
        );
        self.event(text, Level::Info);
        self.updates.found = Some(found.clone());
        self.handle.update(move |s, _| s.update = Some(found));
    }

    /// Downloads and installs the release found earlier, then quits so the new
    /// version (already started, waiting for this process) takes over.
    pub(super) fn install_update(&self) {
        let Some(download) = self.updates.found.as_ref().and_then(|f| f.download.clone()) else {
            return;
        };
        self.event(t!("ev_update_installing"), Level::Info);
        self.handle
            .update(|s, _| s.update_stage = update::Stage::Installing);
        let handle = self.handle.clone();
        std::thread::spawn(move || match update::install(&download) {
            Ok(()) => handle.update(|_, tx| tx.quit()),
            Err(error) => handle.update(move |s, _| s.update_stage = update::Stage::Failed(error)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_check_waits_and_then_repeats() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        assert!(!s.due(t0 + Duration::from_secs(10)));
        assert!(s.due(t0 + FIRST));
        s.done(t0 + FIRST, true);
        assert!(!s.due(t0 + FIRST + Duration::from_secs(3600)));
        assert!(s.due(t0 + FIRST + EVERY));
    }

    #[test]
    fn a_failed_check_is_retried_sooner() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        s.done(t0, false);
        assert!(s.due(t0 + RETRY));
        assert!(RETRY < EVERY);
    }
}

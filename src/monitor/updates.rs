//! The periodic "is there a newer release?" check. The network call runs on
//! its own short-lived thread so a slow connection never delays a check of
//! the VPN; the answer reaches the window through its handle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use super::Monitor;
use super::notify::notify;
use crate::t;
use crate::ui::Level;
use crate::update;

/// The first look: right at start, once.
const FIRST: Duration = Duration::ZERO;
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

    /// Makes the next check due at once (a manual check).
    pub(super) fn now(&mut self, now: Instant) {
        self.next = now;
    }
}

type Answer = Result<Option<update::Available>, String>;

/// The check in flight, if any, and when the next one is due.
pub(super) struct Updates {
    schedule: Schedule,
    running: Option<Receiver<Answer>>,
    /// The release the check found, for "update and restart".
    found: Option<update::Available>,
    /// A check was asked for by hand: it ignores the setting, and whoever sees
    /// its answer first (the check's thread, or this loop) reports "up to
    /// date" or a failure to the window, exactly once (`swap(false)`).
    manual: Arc<AtomicBool>,
}

impl Updates {
    pub(super) fn new(now: Instant) -> Updates {
        Updates {
            schedule: Schedule::new(now),
            running: None,
            found: None,
            manual: Arc::new(AtomicBool::new(false)),
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
                    if self.updates.manual.swap(false, Ordering::SeqCst) {
                        report_manual(&self.handle, &answer);
                    }
                    self.updates.schedule.done(now, answer.is_ok());
                    if let Ok(Some(found)) = answer {
                        self.found_update(found);
                    }
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.updates.running = None;
                    if self.updates.manual.swap(false, Ordering::SeqCst) {
                        report_manual(&self.handle, &Err("no answer".into()));
                    }
                    self.updates.schedule.done(now, false);
                }
            }
            return;
        }
        if !self.updates.schedule.due(now) {
            return;
        }
        if !self.updates.manual.load(Ordering::SeqCst) && !self.cfg.lock().unwrap().check_updates {
            self.updates.schedule.done(now, true);
            return;
        }
        let (tx, rx) = channel();
        self.updates.running = Some(rx);
        // A manual check answers in the window at once; the monitor picks the
        // result up on its next round for the notice and the schedule.
        let (manual, handle) = (self.updates.manual.clone(), self.handle.clone());
        std::thread::spawn(move || {
            let answer = update::check(crate::VERSION);
            if manual.swap(false, Ordering::SeqCst) {
                report_manual(&handle, &answer);
            }
            let _ = tx.send(answer);
        });
    }

    /// The Settings button. A check that already finished is collected first
    /// (its answer, seconds old, is the reply); otherwise one starts now, or
    /// the one in flight reports when it ends.
    pub(super) fn check_updates_now(&mut self) {
        self.updates.manual.store(true, Ordering::SeqCst);
        self.maybe_check_updates();
        if self.updates.running.is_none() && self.updates.manual.load(Ordering::SeqCst) {
            self.updates.schedule.now(Instant::now());
            self.maybe_check_updates();
        }
    }

    fn found_update(&mut self, found: update::Available) {
        // The same release again (the 12-hour check, "Check now"): the row in
        // Settings is enough, no second notification.
        if self.updates.found.as_ref().map(|f| &f.version) == Some(&found.version) {
            self.updates.found = Some(found.clone());
            self.handle.update(move |s, _| s.update = Some(found));
            return;
        }
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

/// Tells the window how a manual check ended (and shows a release it found).
fn report_manual(handle: &rune::AppHandle<crate::ui::Ui>, answer: &Answer) {
    let state = manual_result(answer, crate::config::now_unix());
    let found = answer.as_ref().ok().cloned().flatten();
    handle.update(move |s, _| {
        s.update_check = state;
        if found.is_some() {
            s.update = found;
        }
    });
}

/// What the Settings button says after a manual check.
fn manual_result(answer: &Answer, at: u64) -> crate::ui::UpdateCheck {
    use crate::ui::UpdateCheck;
    match answer {
        Ok(Some(_)) => UpdateCheck::Idle, // the "new version" row says it
        Ok(None) => UpdateCheck::UpToDate(at),
        Err(_) => UpdateCheck::Failed(at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_check_reports_up_to_date_or_failure() {
        use crate::ui::UpdateCheck;
        assert_eq!(manual_result(&Ok(None), 7), UpdateCheck::UpToDate(7));
        assert_eq!(
            manual_result(&Err("offline".into()), 7),
            UpdateCheck::Failed(7)
        );
    }

    #[test]
    fn a_manual_check_is_due_at_once() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        s.now(t0);
        assert!(s.due(t0));
    }

    #[test]
    fn the_first_check_runs_at_start_and_then_repeats() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        assert!(s.due(t0), "the first check runs at start");
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

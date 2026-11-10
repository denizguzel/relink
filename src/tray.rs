//! The notification-area icon: status colour, tooltip and menu. Rune's tray
//! can't change its icon in place, so a new colour rebuilds the tray. Every
//! change comes from the monitor thread, never from the tray's own thread
//! (dropping a tray joins that thread).

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use rune::AppHandle;

use crate::brand;
use crate::diagnose::Status;
use crate::t;
use crate::ui::{Cmd, Ui};

struct Inner {
    tray: Option<rune::Tray>,
    shown: Option<Option<Status>>,
}

pub struct TrayHost {
    inner: Mutex<Inner>,
    handle: AppHandle<Ui>,
    cmd: Sender<Cmd>,
}

/// The tray menu. Fixing and checking mean nothing while the VPN is down (or
/// just coming up); the pause entry flips to "resume" while paused.
fn menu(status: Option<Status>, summary: &str, paused: bool) -> Vec<rune::TrayItem> {
    // The first line says what the icon's colour means.
    let mut items = vec![
        rune::TrayItem::action("status", summary).enabled(false),
        rune::TrayItem::action("open", t!("open_window")),
        rune::TrayItem::Separator,
    ];
    // Restarting the network service is only offered when something is stuck.
    if matches!(status, Some(Status::Stuck | Status::MissingRoute)) {
        items.push(rune::TrayItem::action("fix", t!("fix_now")));
    }
    if status != Some(Status::VpnDown) {
        items.push(rune::TrayItem::action("check", t!("check_now")));
    }
    if items.len() > 3 {
        items.push(rune::TrayItem::Separator);
    }
    items.push(if paused {
        rune::TrayItem::action("resume", t!("resume_notifications"))
    } else {
        rune::TrayItem::action("pause", t!("pause_notifications"))
    });
    items.push(rune::TrayItem::action("quit", t!("quit")));
    items
}

impl TrayHost {
    /// Creates the tray in the "checking" state. `None` when the system has
    /// no tray (the window then quits on close).
    pub fn new(handle: AppHandle<Ui>, cmd: Sender<Cmd>) -> Option<Arc<TrayHost>> {
        let host = Arc::new(TrayHost {
            inner: Mutex::new(Inner {
                tray: None,
                shown: None,
            }),
            handle,
            cmd,
        });
        host.set(None, format!("Re-Link — {}", t!("status_checking")), false)
            .then_some(host)
    }

    /// Shows `status` with `tooltip`; rebuilds the icon only when the colour
    /// changes. Returns whether a tray exists.
    pub fn set(&self, status: Option<Status>, tooltip: String, paused: bool) -> bool {
        let summary = tooltip
            .strip_prefix("Re-Link — ")
            .unwrap_or(&tooltip)
            .to_string();
        let summary = summary.as_str();
        let mut inner = self.inner.lock().unwrap();
        if inner.shown == Some(status)
            && let Some(tray) = &inner.tray
        {
            tray.set_tooltip(tooltip);
            tray.set_menu(menu(status, summary, paused));
            return true;
        }
        inner.tray = None; // removes the old icon first
        let (handle, cmd) = (self.handle.clone(), self.cmd.clone());
        let icon = rune::TrayIcon::rgba(
            brand::TRAY_SIZE,
            brand::TRAY_SIZE,
            brand::tray_rgba(status).to_vec(),
        );
        let spec = rune::TraySpec::new("relink", "Re-Link", icon)
            .tooltip(tooltip)
            .menu(menu(status, summary, paused));
        match rune::Tray::new(spec, move |event| match event {
            rune::TrayEvent::Activate { .. } => handle.open(),
            rune::TrayEvent::Menu(id) => match id.as_str() {
                "open" => handle.open(),
                "fix" => {
                    let _ = cmd.send(Cmd::FixNow);
                }
                "check" => {
                    let _ = cmd.send(Cmd::CheckNow);
                }
                "pause" => {
                    let _ = cmd.send(Cmd::Pause(crate::ui::pause::PAUSE_SECS));
                }
                "resume" => {
                    let _ = cmd.send(Cmd::Resume);
                }
                "quit" => handle.quit(),
                _ => {}
            },
            _ => {}
        }) {
            Ok(tray) => {
                inner.tray = Some(tray);
                inner.shown = Some(status);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(items: &[rune::TrayItem]) -> Vec<String> {
        items
            .iter()
            .map(|i| match i {
                rune::TrayItem::Action { id, .. } => id.clone(),
                _ => "-".to_string(),
            })
            .collect()
    }

    #[test]
    fn healthy_menu_offers_check_pause_and_quit() {
        let m = menu(Some(Status::Ok), "All good", false);
        assert_eq!(
            ids(&m),
            ["status", "open", "-", "check", "-", "pause", "quit"]
        );
    }

    #[test]
    fn a_stuck_connection_offers_the_fix() {
        let m = menu(Some(Status::Stuck), "2 stuck connections", false);
        assert_eq!(
            ids(&m),
            ["status", "open", "-", "fix", "check", "-", "pause", "quit"]
        );
    }

    #[test]
    fn the_first_line_is_a_disabled_status_line() {
        let m = menu(Some(Status::Ok), "All good", false);
        assert!(
            matches!(&m[0], rune::TrayItem::Action { id, label, enabled: false } if id == "status" && label == "All good")
        );
    }

    #[test]
    fn no_vpn_hides_fix_and_check() {
        let m = menu(Some(Status::VpnDown), "VPN is not connected", false);
        assert_eq!(ids(&m), ["status", "open", "-", "pause", "quit"]);
    }

    #[test]
    fn while_routes_arrive_only_check_is_offered() {
        let m = menu(Some(Status::Settling), "Connecting", false);
        assert_eq!(
            ids(&m),
            ["status", "open", "-", "check", "-", "pause", "quit"]
        );
    }

    #[test]
    fn pause_flips_to_resume_while_paused() {
        assert!(ids(&menu(Some(Status::Ok), "x", true)).contains(&"resume".to_string()));
        assert!(!ids(&menu(Some(Status::Ok), "x", true)).contains(&"pause".to_string()));
    }
}

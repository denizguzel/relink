//! The one-line status texts of the tray, the sidebar and the overview.

use crate::diagnose::{Report, Status};
use crate::t;

/// The status line shown in the tray tooltip, the sidebar and the overview.
pub fn status_text(status: Option<Status>, report: Option<&Report>) -> String {
    match (status, report) {
        (None, _) | (_, None) => t!("status_checking"),
        (Some(Status::Ok), _) => t!("status_ok"),
        (Some(Status::VpnDown), _) => t!("status_vpn_down"),
        (Some(Status::Settling), _) => t!("status_settling"),
        (Some(Status::Stuck), Some(r)) => t!("status_stuck", count = r.stuck.len()),
        (Some(Status::MissingRoute), Some(r)) => {
            t!("status_missing_short", count = r.missing_hosts().len())
        }
    }
}

/// Like [`status_text`], but short enough for the tray tooltip (128
/// characters) and the sidebar footer.
pub fn short_status(report: Option<&Report>) -> String {
    match report.map(Report::status) {
        Some(Status::MissingRoute) => t!(
            "status_missing_short",
            count = report.map_or(0, |r| r.missing_hosts().len())
        ),
        s => status_text(s, report),
    }
}

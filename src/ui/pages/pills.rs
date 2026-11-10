//! Coloured status pills for connections and hosts.

use rune::El;
use rune_kit::Kit;

use crate::diagnose::{ConnKind, HostKind};
use crate::iconart;
use crate::t;
use crate::ui::parts::{self, pill};

pub(super) fn kind_pill(kind: ConnKind, k: &Kit) -> El {
    let (label, color) = match kind {
        ConnKind::Vpn => (t!("kind_vpn"), parts::rgb(iconart::GREEN)),
        ConnKind::Internet => (t!("kind_internet"), parts::rgb(iconart::GRAY)),
        ConnKind::Stuck => (t!("kind_stuck"), parts::rgb(iconart::AMBER)),
        ConnKind::Unknown => (t!("kind_unknown"), parts::rgb(iconart::GRAY)),
    };
    pill(label, color, k)
}

pub(super) fn host_pill(kind: Option<HostKind>, k: &Kit) -> El {
    let (label, color) = match kind {
        Some(HostKind::Vpn) => (t!("kind_vpn"), parts::rgb(iconart::GREEN)),
        Some(HostKind::Missing) => (t!("kind_missing"), parts::rgb(iconart::RED)),
        Some(HostKind::Internet) => (t!("kind_internet"), parts::rgb(iconart::GRAY)),
        Some(HostKind::Unknown) => (t!("kind_unknown"), parts::rgb(iconart::GRAY)),
        None => (t!("unresolved"), parts::rgb(iconart::GRAY)),
    };
    pill(label, color, k)
}

/// The pill of one address's path: through the VPN, the home network, or none.
pub(super) fn path_pill(path: super::hosts::Path, k: &Kit) -> El {
    use super::hosts::Path;
    let (label, color) = match path {
        Path::Vpn => (t!("path_vpn"), parts::rgb(iconart::GREEN)),
        Path::Home => (t!("path_home"), parts::rgb(iconart::AMBER)),
        Path::NoRoute => (t!("path_none"), parts::rgb(iconart::GRAY)),
    };
    pill(label, color, k)
}

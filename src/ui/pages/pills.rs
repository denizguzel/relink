//! Coloured status pills for connections and hosts.

use rune::El;
use rune_kit::Kit;

use crate::brand;
use crate::diagnose::{ConnKind, HostKind};
use crate::t;
use crate::ui::parts::{self, pill};

pub(super) fn kind_pill(kind: ConnKind, k: &Kit) -> El {
    let (label, color) = match kind {
        ConnKind::Vpn => (t!("kind_vpn"), parts::rgb(brand::GREEN)),
        ConnKind::Internet => (t!("kind_internet"), parts::rgb(brand::GRAY)),
        ConnKind::Stuck => (t!("kind_stuck"), parts::rgb(brand::AMBER)),
        ConnKind::Unknown => (t!("kind_unknown"), parts::rgb(brand::GRAY)),
    };
    pill(label, color, k)
}

pub(super) fn host_pill(kind: Option<HostKind>, k: &Kit) -> El {
    let (label, color) = match kind {
        Some(HostKind::Vpn) => (t!("kind_vpn"), parts::rgb(brand::GREEN)),
        Some(HostKind::Missing) => (t!("kind_missing"), parts::rgb(brand::RED)),
        Some(HostKind::Internet) => (t!("kind_internet"), parts::rgb(brand::GRAY)),
        Some(HostKind::Unknown) => (t!("kind_unknown"), parts::rgb(brand::GRAY)),
        None => (t!("unresolved"), parts::rgb(brand::GRAY)),
    };
    pill(label, color, k)
}

/// The pill of one address's path: through the VPN, the home network, or none.
pub(super) fn path_pill(path: super::hosts::Path, k: &Kit) -> El {
    use super::hosts::Path;
    let (label, color) = match path {
        Path::Vpn => (t!("path_vpn"), parts::rgb(brand::GREEN)),
        Path::Home => (t!("path_home"), parts::rgb(brand::AMBER)),
        Path::NoRoute => (t!("path_none"), parts::rgb(brand::GRAY)),
    };
    pill(label, color, k)
}

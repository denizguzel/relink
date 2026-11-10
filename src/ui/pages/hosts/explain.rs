//! Why a host is in its state: what each of its addresses does and a sentence
//! about the host as a whole. Pure, so the rules can be tested.

use std::net::Ipv4Addr;

use crate::diagnose::{HostKind, Route};

/// Where traffic to one address goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// Through the VPN adapter.
    Vpn,
    /// Through another adapter: the home network.
    Home,
    /// No route could be found.
    NoRoute,
}

/// Classifies a route against the VPN adapter's interface index (None: no VPN).
pub fn path(route: Option<Route>, vpn_index: Option<u32>) -> Path {
    match route {
        None => Path::NoRoute,
        Some(r) if vpn_index == Some(r.if_index) => Path::Vpn,
        Some(_) => Path::Home,
    }
}

/// "/32 · if 20", "default · if 7", or a dash without a route.
pub fn route_text(route: Option<Route>) -> String {
    match route {
        None => "—".to_string(),
        Some(r) if r.prefix_len == 0 => format!("default · if {}", r.if_index),
        Some(r) => format!("/{} · if {}", r.prefix_len, r.if_index),
    }
}

/// The sentence explaining a host's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// Every address goes through the VPN.
    Vpn,
    /// It should use the VPN but some addresses leave over the home network.
    Missing,
    /// It does not use a VPN route; it goes over the normal internet.
    Internet,
    /// The VPN is down, so nothing can be said.
    NoVpn,
    /// The name did not resolve.
    Unresolved,
}

pub fn why(kind: Option<HostKind>) -> Why {
    match kind {
        None => Why::Unresolved,
        Some(HostKind::Vpn) => Why::Vpn,
        Some(HostKind::Missing) => Why::Missing,
        Some(HostKind::Internet) => Why::Internet,
        Some(HostKind::Unknown) => Why::NoVpn,
    }
}

/// How many of a host's addresses leave over the home network.
pub fn home_count(routes: &[(Ipv4Addr, Option<Route>)], vpn_index: Option<u32>) -> usize {
    routes
        .iter()
        .filter(|(_, r)| path(*r, vpn_index) == Path::Home)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(if_index: u32, prefix_len: u8) -> Option<Route> {
        Some(Route {
            if_index,
            prefix_len,
            source: None,
        })
    }

    #[test]
    fn a_route_on_the_vpn_adapter_is_a_vpn_path() {
        assert_eq!(path(route(20, 32), Some(20)), Path::Vpn);
        assert_eq!(path(route(7, 0), Some(20)), Path::Home);
        assert_eq!(path(None, Some(20)), Path::NoRoute);
    }

    #[test]
    fn without_a_vpn_adapter_nothing_is_on_the_vpn() {
        assert_eq!(path(route(20, 32), None), Path::Home);
    }

    #[test]
    fn routes_are_described_briefly() {
        assert_eq!(route_text(route(20, 32)), "/32 · if 20");
        assert_eq!(route_text(route(7, 0)), "default · if 7");
        assert_eq!(route_text(None), "—");
    }

    #[test]
    fn every_host_kind_has_an_explanation() {
        assert_eq!(why(Some(HostKind::Vpn)), Why::Vpn);
        assert_eq!(why(Some(HostKind::Missing)), Why::Missing);
        assert_eq!(why(Some(HostKind::Internet)), Why::Internet);
        assert_eq!(why(Some(HostKind::Unknown)), Why::NoVpn);
        assert_eq!(why(None), Why::Unresolved);
    }

    #[test]
    fn counts_the_addresses_that_leave_over_the_home_network() {
        let ip = |n| Ipv4Addr::new(1, 1, 1, n);
        let routes = [
            (ip(1), route(20, 32)),
            (ip(2), route(7, 0)),
            (ip(3), route(7, 0)),
            (ip(4), None),
        ];
        assert_eq!(home_count(&routes, Some(20)), 2);
        assert_eq!(home_count(&[], Some(20)), 0);
    }
}

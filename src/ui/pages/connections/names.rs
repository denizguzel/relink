//! Which watched hosts an address belongs to, so a connection can say
//! "app.example.com" next to its IP. Only watched hosts are known; an address
//! outside them (a CDN, say) has no name.

use std::net::Ipv4Addr;

use crate::ui::HostRow;

/// The watched hosts that resolve to `ip`, alphabetically.
pub fn hosts_of(ip: Ipv4Addr, hosts: &[HostRow]) -> Vec<&str> {
    let mut names: Vec<&str> = hosts
        .iter()
        .filter(|h| h.ips.contains(&ip))
        .map(|h| h.host.as_str())
        .collect();
    names.sort_unstable();
    names
}

/// "first.example" or "first.example +2": a shared address (a load balancer
/// serves many hosts) shows one name and how many more there are.
pub fn label(names: &[&str]) -> Option<String> {
    match names {
        [] => None,
        [only] => Some((*only).to_string()),
        [first, rest @ ..] => Some(format!("{first} +{}", rest.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Sources;

    fn host(name: &str, ips: &[[u8; 4]]) -> HostRow {
        HostRow {
            host: name.into(),
            ips: ips.iter().map(|o| Ipv4Addr::from(*o)).collect(),
            routes: vec![],
            kind: None,
            sources: Sources::default(),
        }
    }

    #[test]
    fn finds_every_host_behind_an_address_alphabetically() {
        let hosts = [
            host("b.example", &[[1, 1, 1, 1], [2, 2, 2, 2]]),
            host("a.example", &[[1, 1, 1, 1]]),
            host("c.example", &[[3, 3, 3, 3]]),
        ];
        assert_eq!(
            hosts_of(Ipv4Addr::new(1, 1, 1, 1), &hosts),
            ["a.example", "b.example"]
        );
        assert_eq!(hosts_of(Ipv4Addr::new(3, 3, 3, 3), &hosts), ["c.example"]);
        assert!(hosts_of(Ipv4Addr::new(9, 9, 9, 9), &hosts).is_empty());
        assert!(hosts_of(Ipv4Addr::new(1, 1, 1, 1), &[]).is_empty());
    }

    #[test]
    fn a_shared_address_shows_one_name_and_a_count() {
        assert_eq!(label(&[]), None);
        assert_eq!(label(&["a.example"]), Some("a.example".to_string()));
        assert_eq!(
            label(&["a.example", "b.example", "c.example"]),
            Some("a.example +2".to_string())
        );
    }
}

//! Which connections the page shows and how they are grouped.

use std::collections::BTreeMap;

use crate::diagnose::{Conn, ConnKind, Route};

/// One row of the report: a connection, its route and its classification.
pub type Item = (Conn, Option<Route>, ConnKind);

/// How bad a kind is; groups and rows list the worst first.
pub fn rank(kind: ConnKind) -> u8 {
    match kind {
        ConnKind::Stuck => 0,
        ConnKind::Vpn => 1,
        ConnKind::Unknown => 2,
        ConnKind::Internet => 3,
    }
}

/// Whether the filter (0 all, 1 stuck, 2 VPN, 3 internet) keeps a kind.
pub fn keep(filter: usize, kind: ConnKind) -> bool {
    match filter {
        1 => kind == ConnKind::Stuck,
        2 => kind == ConnKind::Vpn,
        3 => kind == ConnKind::Internet,
        _ => true,
    }
}

/// The connections that pass `filter`, one group per browser process: the
/// group with the worst connection first, each group worst first.
pub fn group(all: Vec<Item>, filter: usize) -> Vec<(String, Vec<Item>)> {
    let mut by_process: BTreeMap<String, Vec<Item>> = BTreeMap::new();
    for item in all.into_iter().filter(|(_, _, kind)| keep(filter, *kind)) {
        by_process
            .entry(item.0.process.clone())
            .or_default()
            .push(item);
    }
    let mut groups: Vec<(String, Vec<Item>)> = by_process.into_iter().collect();
    for (_, conns) in &mut groups {
        conns.sort_by_key(|(conn, _, kind)| (rank(*kind), conn.remote, conn.pid));
    }
    groups.sort_by_key(|(name, conns)| {
        (
            conns.iter().map(|c| rank(c.2)).min().unwrap_or(u8::MAX),
            name.clone(),
        )
    });
    groups
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn item(process: &str, pid: u32, remote: u8, kind: ConnKind) -> Item {
        let conn = Conn {
            pid,
            process: process.into(),
            local: Ipv4Addr::new(192, 168, 1, 2),
            local_port: 5000,
            remote: Ipv4Addr::new(1, 1, 1, remote),
            remote_port: 443,
        };
        (conn, None, kind)
    }

    fn names(groups: &[(String, Vec<Item>)]) -> Vec<&str> {
        groups.iter().map(|g| g.0.as_str()).collect()
    }

    #[test]
    fn groups_by_process_with_the_worst_first() {
        let all = vec![
            item("chrome.exe", 1, 1, ConnKind::Internet),
            item("firefox.exe", 2, 1, ConnKind::Stuck),
            item("chrome.exe", 1, 2, ConnKind::Vpn),
            item("brave.exe", 3, 1, ConnKind::Internet),
        ];
        let g = group(all, 0);
        // Firefox has a stuck one; chrome's best is VPN; brave only internet.
        assert_eq!(names(&g), ["firefox.exe", "chrome.exe", "brave.exe"]);
        let chrome = &g[1].1;
        assert_eq!(
            chrome.iter().map(|i| i.2).collect::<Vec<_>>(),
            [ConnKind::Vpn, ConnKind::Internet]
        );
    }

    #[test]
    fn filters_keep_only_the_chosen_kind() {
        let all = || {
            vec![
                item("a.exe", 1, 1, ConnKind::Stuck),
                item("a.exe", 1, 2, ConnKind::Vpn),
                item("b.exe", 2, 1, ConnKind::Internet),
            ]
        };
        assert_eq!(group(all(), 1).iter().map(|g| g.1.len()).sum::<usize>(), 1);
        assert_eq!(names(&group(all(), 2)), ["a.exe"]);
        assert_eq!(names(&group(all(), 3)), ["b.exe"]);
        assert_eq!(group(all(), 0).iter().map(|g| g.1.len()).sum::<usize>(), 3);
    }

    #[test]
    fn rows_within_a_group_sort_by_rank_then_address() {
        let all = vec![
            item("a.exe", 1, 9, ConnKind::Vpn),
            item("a.exe", 1, 3, ConnKind::Vpn),
            item("a.exe", 1, 5, ConnKind::Stuck),
        ];
        let g = group(all, 0);
        let remotes: Vec<u8> = g[0].1.iter().map(|i| i.0.remote.octets()[3]).collect();
        assert_eq!(remotes, [5, 3, 9]);
    }

    #[test]
    fn nothing_passing_the_filter_gives_no_groups() {
        assert!(group(vec![item("a.exe", 1, 1, ConnKind::Internet)], 1).is_empty());
        assert!(group(Vec::new(), 0).is_empty());
    }
}

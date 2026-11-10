//! Which hosts the page lists, and in what order.

use crate::diagnose::HostKind;
use crate::ui::HostRow;

/// How bad a host is; the ones needing attention come first.
pub fn rank(kind: Option<HostKind>) -> u8 {
    match kind {
        Some(HostKind::Missing) => 0,
        Some(HostKind::Vpn) => 1,
        Some(HostKind::Unknown) => 2,
        Some(HostKind::Internet) => 3,
        None => 4,
    }
}

/// The hosts whose name contains `filter` (case-insensitive), worst first and
/// alphabetical within a kind.
pub fn visible<'a>(all: &'a [HostRow], filter: &str) -> Vec<&'a HostRow> {
    let filter = filter.trim().to_lowercase();
    let mut shown: Vec<&HostRow> = all
        .iter()
        .filter(|h| filter.is_empty() || h.host.contains(&filter))
        .collect();
    shown.sort_by(|a, b| (rank(a.kind), &a.host).cmp(&(rank(b.kind), &b.host)));
    shown
}

/// The host whose detail is open after `clicked` is pressed: pressing the open
/// one closes it, pressing another opens that one instead.
pub fn toggled(open: &Option<String>, clicked: &str) -> Option<String> {
    if open.as_deref() == Some(clicked) {
        None
    } else {
        Some(clicked.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Sources;

    fn host(name: &str, kind: Option<HostKind>) -> HostRow {
        HostRow {
            host: name.into(),
            ips: vec![],
            routes: vec![],
            kind,
            sources: Sources::default(),
        }
    }

    fn names(rows: &[&HostRow]) -> Vec<String> {
        rows.iter().map(|h| h.host.clone()).collect()
    }

    #[test]
    fn attention_first_then_alphabetical() {
        let all = vec![
            host("b.internet", Some(HostKind::Internet)),
            host("z.vpn", Some(HostKind::Vpn)),
            host("a.vpn", Some(HostKind::Vpn)),
            host("m.missing", Some(HostKind::Missing)),
            host("u.unresolved", None),
        ];
        assert_eq!(
            names(&visible(&all, "")),
            ["m.missing", "a.vpn", "z.vpn", "b.internet", "u.unresolved"]
        );
    }

    #[test]
    fn filter_matches_substrings_ignoring_case_and_padding() {
        let all = vec![
            host("metrics.etiya.com", None),
            host("git.etiya.com", None),
            host("ci.example.org", None),
        ];
        assert_eq!(
            names(&visible(&all, "  ETIYA ")),
            ["git.etiya.com", "metrics.etiya.com"]
        );
        assert!(visible(&all, "nothing").is_empty());
    }

    #[test]
    fn pressing_a_row_opens_it_and_pressing_again_closes_it() {
        let none = None;
        let open = toggled(&none, "a.example");
        assert_eq!(open.as_deref(), Some("a.example"));
        assert_eq!(toggled(&open, "a.example"), None);
        assert_eq!(toggled(&open, "b.example").as_deref(), Some("b.example"));
    }
}

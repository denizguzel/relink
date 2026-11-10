//! Semantic versions (`1.2.3`, `v1.2.3-rc.1`), just enough to tell whether a
//! release is newer than the running build.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ident {
    Num(u64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    /// Empty for a final release; a pre-release sorts before it.
    pre: Vec<Ident>,
}

impl Version {
    /// Parses `1.2.3`, with an optional leading `v`, `-pre.release` and
    /// `+build` (ignored). Anything else is not a version.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut nums = core.split('.');
        let mut core = [0; 3];
        for slot in &mut core {
            *slot = nums.next()?.parse().ok()?;
        }
        if nums.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => vec![],
            Some(pre) => pre
                .split('.')
                .map(|part| {
                    if part.is_empty()
                        || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    {
                        return None;
                    }
                    Some(match part.parse() {
                        Ok(n) => Ident::Num(n),
                        Err(_) => Ident::Text(part.to_string()),
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        };
        Some(Version { core, pre })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core
            .cmp(&other.core)
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ident {
    /// Numbers sort before text, as in the semver rules.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Ident::Num(a), Ident::Num(b)) => a.cmp(b),
            (Ident::Text(a), Ident::Text(b)) => a.cmp(b),
            (Ident::Num(_), Ident::Text(_)) => Ordering::Less,
            (Ident::Text(_), Ident::Num(_)) => Ordering::Greater,
        }
    }
}

/// Whether `candidate` is a newer version than `current`. Unparseable input
/// is never newer: a malformed tag must not nag anyone.
pub fn is_newer(current: &str, candidate: &str) -> bool {
    match (Version::parse(current), Version::parse(candidate)) {
        (Some(current), Some(candidate)) => candidate > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags_with_or_without_v() {
        assert_eq!(Version::parse("v1.2.3"), Version::parse("1.2.3"));
        assert!(Version::parse("1.2").is_none());
        assert!(Version::parse("1.2.3.4").is_none());
        assert!(Version::parse("latest").is_none());
        assert!(Version::parse("1.2.x").is_none());
        assert!(Version::parse("1.2.3-").is_none());
    }

    #[test]
    fn compares_numbers_not_text() {
        assert!(is_newer("0.2.0", "0.10.0"), "0.10 is newer than 0.2");
        assert!(is_newer("1.9.9", "2.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.1.0", "1.0.9"));
    }

    #[test]
    fn a_release_beats_its_prereleases() {
        assert!(is_newer("0.3.0-rc.1", "0.3.0"));
        assert!(!is_newer("0.3.0", "0.3.0-rc.2"));
        assert!(is_newer("0.3.0-rc.1", "0.3.0-rc.2"));
        assert!(is_newer("0.3.0-rc.2", "0.3.0-rc.10"));
        assert!(is_newer("0.3.0-alpha", "0.3.0-beta"));
        assert!(
            is_newer("0.3.0-1", "0.3.0-alpha"),
            "numbers sort before text"
        );
    }

    #[test]
    fn build_metadata_is_ignored() {
        assert!(!is_newer("1.0.0+abc", "1.0.0+def"));
    }

    #[test]
    fn garbage_is_never_newer() {
        assert!(!is_newer("0.2.0", "nightly"));
        assert!(!is_newer("dev", "1.0.0"));
    }
}

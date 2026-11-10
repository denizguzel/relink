//! Reading GitHub's "latest release" answer and deciding whether it is news.

use serde::Deserialize;

use super::REPO;
use super::asset::{SUMS, is_download_url, platform_name};
use super::version::is_newer;

/// A newer release than the running one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Available {
    /// The release tag without its leading `v`, e.g. `0.3.0`.
    pub version: String,
    /// The release page, where the downloads are.
    pub url: String,
    /// What the in-app update needs; `None` when this release has no file for
    /// this platform (then the user is sent to the release page).
    pub download: Option<Download>,
}

/// The files of an in-app update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Download {
    /// The executable's file name in the release, the key in `SHA256SUMS`.
    pub name: String,
    pub asset_url: String,
    pub sums_url: String,
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    assets: Vec<RawAsset>,
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Whether `url` is a page of this project's repository and contains only
/// characters that are safe to hand to the desktop's URL opener.
pub fn is_release_url(url: &str) -> bool {
    let prefix = format!("https://github.com/{REPO}/");
    url.strip_prefix(&prefix).is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'-' | b'_'))
    })
}

/// The newer release in `json`, if there is one. Drafts, prereleases and
/// pages outside the repository are ignored.
pub fn decide(current: &str, json: &str) -> Result<Option<Available>, String> {
    decide_for(current, json, platform_name)
}

/// `decide` for the update file name `asset_name` gives (a parameter so tests
/// do not depend on the platform they run on).
fn decide_for(
    current: &str,
    json: &str,
    asset_name: impl Fn(&str) -> Option<String>,
) -> Result<Option<Available>, String> {
    let raw: Raw = serde_json::from_str(json).map_err(|e| format!("release answer: {e}"))?;
    if raw.draft || raw.prerelease || !is_release_url(&raw.html_url) {
        return Ok(None);
    }
    if !is_newer(current, &raw.tag_name) {
        return Ok(None);
    }
    let version = raw.tag_name.trim_start_matches('v').to_string();
    let find = |name: &str| {
        raw.assets
            .iter()
            .find(|a| a.name == name && is_download_url(&a.browser_download_url))
            .map(|a| a.browser_download_url.clone())
    };
    let download = asset_name(&version).and_then(|name| {
        Some(Download {
            asset_url: find(&name)?,
            sums_url: find(SUMS)?,
            name,
        })
    });
    Ok(Some(Available {
        version,
        url: raw.html_url,
        download,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(tag: &str, url: &str, extra: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","html_url":"{url}","name":"x"{extra}}}"#)
    }
    const PAGE: &str = "https://github.com/denizguzel/relink/releases/tag/v0.3.0";

    #[test]
    fn a_newer_tag_is_available() {
        let got = decide("0.2.0", &answer("v0.3.0", PAGE, "")).unwrap();
        assert_eq!(
            got,
            Some(Available {
                version: "0.3.0".into(),
                url: PAGE.into(),
                download: None
            })
        );
    }

    #[test]
    fn the_same_or_an_older_tag_is_not() {
        assert_eq!(decide("0.3.0", &answer("v0.3.0", PAGE, "")).unwrap(), None);
        assert_eq!(decide("0.4.0", &answer("v0.3.0", PAGE, "")).unwrap(), None);
    }

    #[test]
    fn drafts_and_prereleases_are_skipped() {
        let draft = answer("v0.3.0", PAGE, r#","draft":true"#);
        let pre = answer("v0.3.0", PAGE, r#","prerelease":true"#);
        assert_eq!(decide("0.2.0", &draft).unwrap(), None);
        assert_eq!(decide("0.2.0", &pre).unwrap(), None);
    }

    #[test]
    fn only_this_repositorys_pages_are_trusted() {
        for url in [
            "https://evil.example/denizguzel/relink/releases",
            "http://github.com/denizguzel/relink/releases",
            "https://github.com/denizguzel/relink-evil/releases",
            "https://github.com/denizguzel/relink/releases?x=1&y=2",
            "https://github.com/denizguzel/relink/",
            "https://github.com/denizguzel/relink/releases/tag/v1 & calc",
        ] {
            assert!(!is_release_url(url), "{url}");
            assert_eq!(decide("0.2.0", &answer("v9.0.0", url, "")).unwrap(), None);
        }
        assert!(is_release_url(PAGE));
    }

    #[test]
    fn broken_json_is_an_error() {
        assert!(decide("0.2.0", "{not json").is_err());
        assert!(decide("0.2.0", r#"{"message":"Not Found"}"#).is_err());
    }

    const BASE: &str = "https://github.com/denizguzel/relink/releases/download/v0.3.0";

    fn assets(names: &[&str]) -> String {
        let list: Vec<String> = names
            .iter()
            .map(|n| format!(r#"{{"name":"{n}","browser_download_url":"{BASE}/{n}"}}"#))
            .collect();
        format!(r#","assets":[{}]"#, list.join(","))
    }

    fn for_test(version: &str) -> Option<String> {
        Some(format!("relink-{version}-test"))
    }

    #[test]
    fn the_platforms_file_and_the_checksums_make_an_in_app_update() {
        let json = answer(
            "v0.3.0",
            PAGE,
            &assets(&["relink-0.3.0-test", "SHA256SUMS", "other"]),
        );
        let got = decide_for("0.2.0", &json, for_test).unwrap().unwrap();
        assert_eq!(
            got.download,
            Some(Download {
                name: "relink-0.3.0-test".into(),
                asset_url: format!("{BASE}/relink-0.3.0-test"),
                sums_url: format!("{BASE}/SHA256SUMS"),
            })
        );
    }

    #[test]
    fn without_the_file_or_the_checksums_it_falls_back_to_the_page() {
        for names in [
            &["SHA256SUMS"][..],
            &["relink-0.3.0-test"][..],
            &["other"][..],
        ] {
            let json = answer("v0.3.0", PAGE, &assets(names));
            let got = decide_for("0.2.0", &json, for_test).unwrap().unwrap();
            assert_eq!(got.download, None, "{names:?}");
        }
    }

    #[test]
    fn a_file_hosted_elsewhere_is_not_used() {
        let json = answer(
            "v0.3.0",
            PAGE,
            r#","assets":[{"name":"relink-0.3.0-test","browser_download_url":"https://evil.example/x"},{"name":"SHA256SUMS","browser_download_url":"https://evil.example/s"}]"#,
        );
        let got = decide_for("0.2.0", &json, for_test).unwrap().unwrap();
        assert_eq!(got.download, None);
    }
}

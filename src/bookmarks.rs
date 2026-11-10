//! Company hostnames from browser bookmarks, so every user automatically
//! watches the internal sites they use: Chromium "Bookmarks" JSON files, and
//! Firefox's daily bookmark backups (`bookmarkbackups/*.jsonlz4`; the live
//! `places.sqlite` is locked while Firefox runs).

use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize, Default)]
struct Node {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    children: Vec<Node>,
}

/// Distinct, sorted hostnames in a Bookmarks JSON document that equal or end
/// with one of the domain suffixes. IP literals and non-http(s) links are
/// skipped.
pub fn hosts(data: &str, domains: &[String]) -> Result<Vec<String>, String> {
    let doc: serde_json::Value = serde_json::from_str(data).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if let Some(roots) = doc.get("roots").and_then(|r| r.as_object()) {
        for value in roots.values() {
            // "sync_transaction_version" and friends aren't nodes.
            if let Ok(node) = serde_json::from_value::<Node>(value.clone()) {
                walk(&node, domains, &mut out);
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn walk(n: &Node, domains: &[String], out: &mut Vec<String>) {
    if n.kind == "url"
        && let Some(h) = match_host(&n.url, domains)
    {
        out.push(h);
    }
    for c in &n.children {
        walk(c, domains, out);
    }
}

fn match_host(url: &str, domains: &[String]) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = if authority.starts_with('[') {
        return None; // IPv6 literal
    } else {
        authority.split(':').next()?
    }
    .to_ascii_lowercase();
    if host.is_empty() || host.parse::<IpAddr>().is_ok() {
        return None;
    }
    domains
        .iter()
        .map(|d| d.trim().trim_start_matches('.').to_ascii_lowercase())
        .any(|d| !d.is_empty() && (host == d || host.ends_with(&format!(".{d}"))))
        .then_some(host)
}

/// Every profile's Bookmarks file under the given Chromium "User Data"
/// directories.
pub fn files(user_data_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in user_data_dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for e in entries.flatten() {
            let f = e.path().join("Bookmarks");
            if f.is_file() {
                out.push(f);
            }
        }
    }
    out.sort();
    out
}

/// The merged matching hosts of several bookmark files (Chromium JSON or
/// Firefox `.jsonlz4`), ignoring files that can't be read or parsed.
pub fn from_files(files: &[PathBuf], domains: &[String]) -> Vec<String> {
    let mut out: Vec<String> = files
        .iter()
        .filter_map(|f| {
            if f.extension().is_some_and(|e| e == "jsonlz4") {
                firefox_hosts(&mozlz4(&fs::read(f).ok()?).ok()?, domains).ok()
            } else {
                hosts(&fs::read_to_string(f).ok()?, domains).ok()
            }
        })
        .flatten()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Decodes Mozilla's `mozLz40\0` container: an LZ4 block prefixed with its
/// decompressed size.
pub fn mozlz4(data: &[u8]) -> Result<String, String> {
    let body = data.strip_prefix(b"mozLz40\0").ok_or("not a mozLz4 file")?;
    let raw = lz4_flex::block::decompress_size_prepended(body).map_err(|e| e.to_string())?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

/// Matching hosts in a Firefox bookmarks backup (nested nodes with `uri`
/// and `children`).
pub fn firefox_hosts(json: &str, domains: &[String]) -> Result<Vec<String>, String> {
    fn walk(v: &serde_json::Value, domains: &[String], out: &mut Vec<String>) {
        if let Some(h) = v
            .get("uri")
            .and_then(|u| u.as_str())
            .and_then(|u| match_host(u, domains))
        {
            out.push(h);
        }
        for c in v
            .get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
        {
            walk(c, domains, out);
        }
    }
    let doc: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    walk(&doc, domains, &mut out);
    out.sort();
    out.dedup();
    Ok(out)
}

/// The newest bookmark backup of every Firefox profile (stable and Developer
/// Edition share the profile folders) under the given roots.
pub fn firefox_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(profiles) = fs::read_dir(root) else {
            continue;
        };
        for p in profiles.flatten() {
            let Ok(backups) = fs::read_dir(p.path().join("bookmarkbackups")) else {
                continue;
            };
            // Names start with bookmarks-YYYY-MM-DD, so the largest is the newest.
            if let Some(newest) = backups
                .flatten()
                .map(|e| e.path())
                .filter(|f| f.extension().is_some_and(|e| e == "jsonlz4"))
                .max_by_key(|f| f.file_name().map(|n| n.to_os_string()))
            {
                out.push(newest);
            }
        }
    }
    out.sort();
    out
}

/// Firefox profile roots: `%APPDATA%\Mozilla\Firefox\Profiles` on Windows;
/// `~/.mozilla/firefox` and the snap's copy on Linux.
pub fn firefox_roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(|a| {
                vec![
                    PathBuf::from(a)
                        .join("Mozilla")
                        .join("Firefox")
                        .join("Profiles"),
                ]
            })
            .unwrap_or_default()
    } else {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return Vec::new();
        };
        vec![
            home.join(".mozilla").join("firefox"),
            home.join("snap")
                .join("firefox")
                .join("common")
                .join(".mozilla")
                .join("firefox"),
        ]
    }
}

/// Every bookmark source on this machine: Chromium profiles and Firefox
/// backups.
pub fn all_files() -> Vec<PathBuf> {
    let mut f = files(&browser_dirs());
    f.extend(firefox_files(&firefox_roots()));
    f
}

/// Every Chromium browser's profile root (the folder holding `Default`,
/// `Profile 1`, …): Chrome stable/Beta/Dev/Canary, Edge channels, Brave,
/// Chromium, including Ubuntu's snap Chromium.
pub fn browser_dirs() -> Vec<PathBuf> {
    browser_dirs_in(
        &std::env::var_os(if cfg!(windows) {
            "LOCALAPPDATA"
        } else {
            "HOME"
        })
        .map(PathBuf::from)
        .unwrap_or_default(),
    )
}

/// [`browser_dirs`] under `base`: `%LOCALAPPDATA%` on Windows, `$HOME` elsewhere.
pub fn browser_dirs_in(base: &Path) -> Vec<PathBuf> {
    let rel: &[&str] = if cfg!(windows) {
        &[
            "Google/Chrome/User Data",
            "Google/Chrome Beta/User Data",
            "Google/Chrome Dev/User Data",
            "Google/Chrome SxS/User Data",
            "Microsoft/Edge/User Data",
            "Microsoft/Edge Beta/User Data",
            "Microsoft/Edge Dev/User Data",
            "Microsoft/Edge SxS/User Data",
            "BraveSoftware/Brave-Browser/User Data",
            "Chromium/User Data",
        ]
    } else {
        &[
            ".config/google-chrome",
            ".config/google-chrome-beta",
            ".config/google-chrome-unstable",
            ".config/google-chrome-canary",
            ".config/microsoft-edge",
            ".config/microsoft-edge-beta",
            ".config/microsoft-edge-dev",
            ".config/BraveSoftware/Brave-Browser",
            ".config/chromium",
            "snap/chromium/common/chromium",
        ]
    };
    rel.iter()
        .map(|r| {
            r.split('/')
                .fold(base.to_path_buf(), |p, part| p.join(part))
        })
        .filter(|p| p.is_dir())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "checksum": "x",
      "roots": {
        "bookmark_bar": {"type": "folder", "children": [
          {"type": "folder", "name": "Work", "children": [
            {"type": "url", "url": "https://app-t6.etycloudbss.com/login"},
            {"type": "url", "url": "https://APP.etycloudbss.com/"},
            {"type": "url", "url": "https://app.etycloudbss.com:8443/other"},
            {"type": "url", "url": "https://app.etiyacloud.com/x"},
            {"type": "url", "url": "http://212.154.110.19:8080/"},
            {"type": "url", "url": "https://github.com/"},
            {"type": "url", "url": "javascript:alert(1)"},
            {"type": "url", "url": "https://notetycloudbss.com/"}
          ]}
        ]},
        "other": {"type": "folder", "children": [{"type": "url", "url": "https://etycloudbss.com/"}]},
        "sync_transaction_version": "12"
      },
      "version": 1
    }"#;

    fn domains() -> Vec<String> {
        vec!["etycloudbss.com".into(), ".etiyacloud.com".into()]
    }

    #[test]
    fn extracts_matching_hosts() {
        assert_eq!(
            hosts(SAMPLE, &domains()).unwrap(),
            vec![
                "app-t6.etycloudbss.com",
                "app.etiyacloud.com",
                "app.etycloudbss.com",
                "etycloudbss.com"
            ]
        );
    }

    #[test]
    fn finds_existing_browser_variant_dirs_only() {
        let base = std::env::temp_dir().join(format!("relink-dirs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let canary = if cfg!(windows) {
            "Google/Chrome SxS/User Data"
        } else {
            ".config/google-chrome-canary"
        };
        let dir = canary.split('/').fold(base.clone(), |p, part| p.join(part));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(browser_dirs_in(&base), vec![dir]);
    }

    #[test]
    fn reads_firefox_backups_newest_first() {
        let root = std::env::temp_dir().join(format!("relink-ff-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let backups = root
            .join("abcd1234.default-release")
            .join("bookmarkbackups");
        fs::create_dir_all(&backups).unwrap();
        let tree = r#"{"title":"","children":[{"title":"menu","children":[
            {"title":"Work","children":[{"type":"text/x-moz-place","uri":"https://app.etycloudbss.com/x"},
                                        {"type":"text/x-moz-place","uri":"https://github.com/"}]}]},
            {"title":"toolbar","children":[{"uri":"https://app.etiyacloud.com"}]}]}"#;
        let mut moz = b"mozLz40\0".to_vec();
        moz.extend(lz4_flex::block::compress_prepend_size(tree.as_bytes()));
        fs::write(
            backups.join("bookmarks-2026-11-01_10_abc.jsonlz4"),
            b"mozLz40\0broken",
        )
        .unwrap();
        fs::write(backups.join("bookmarks-2026-11-10_12_def.jsonlz4"), &moz).unwrap();
        let files = firefox_files(&[root.clone(), root.join("missing")]);
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("2026-11-10"));
        assert_eq!(
            from_files(&files, &domains()),
            vec!["app.etiyacloud.com", "app.etycloudbss.com"]
        );
        assert!(mozlz4(b"plain json").is_err());
    }

    #[test]
    fn merges_profiles_and_skips_bad_files() {
        let ud = std::env::temp_dir().join(format!("relink-bm-{}", std::process::id()));
        let _ = fs::remove_dir_all(&ud);
        for (name, body) in [
            ("Default", SAMPLE),
            (
                "Profile 1",
                r#"{"roots":{"bookmark_bar":{"type":"folder","children":[{"type":"url","url":"https://logs.etycloudbss.com"}]}}}"#,
            ),
            ("Profile 2", "{broken"),
        ] {
            fs::create_dir_all(ud.join(name)).unwrap();
            fs::write(ud.join(name).join("Bookmarks"), body).unwrap();
        }
        let fs_ = files(&[ud.clone(), ud.join("missing")]);
        assert_eq!(fs_.len(), 3);
        let got = from_files(&fs_, &domains());
        assert_eq!(got.len(), 5);
        assert!(got.contains(&"logs.etycloudbss.com".to_string()));
    }
}

//! Which file of a release is this platform's update, and its checksum.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::REPO;

/// The release's checksum file (`<sha256>  <file>` per line, as `sha256sum` writes it).
pub const SUMS: &str = "SHA256SUMS";

/// The bare executable the release publishes for this platform, which the
/// updater swaps in. `None` where self-update is not offered.
pub fn platform_name(version: &str) -> Option<String> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some(format!("relink-{version}-windows-x64.exe"))
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(format!("relink-{version}-linux-x86_64"))
    } else {
        None
    }
}

/// Whether `url` is a file download of this repository's releases.
pub fn is_download_url(url: &str) -> bool {
    let prefix = format!("https://github.com/{REPO}/releases/download/");
    url.strip_prefix(&prefix).is_some_and(|rest| {
        rest.matches('/').count() == 1
            && rest
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'-' | b'_' | b'+'))
    })
}

/// File name → lowercase SHA-256 from the text of a `SHA256SUMS` file.
pub fn parse_sums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (hash, name) = line.split_once(char::is_whitespace)?;
            let name = name.trim().trim_start_matches('*');
            let ok = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit());
            (ok && !name.is_empty()).then(|| (name.to_string(), hash.to_ascii_lowercase()))
        })
        .collect()
}

/// The SHA-256 of a file, as lowercase hex.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_files_are_parsed_in_both_text_and_binary_mode() {
        let hash = "a".repeat(64);
        let text = format!(
            "{hash}  relink-0.3.0-linux-x86_64\n{}  *other.zip\nnot a line\n",
            "B".repeat(64)
        );
        let sums = parse_sums(&text);
        assert_eq!(sums["relink-0.3.0-linux-x86_64"], hash);
        assert_eq!(
            sums["other.zip"],
            "b".repeat(64),
            "lowercased, star removed"
        );
        assert_eq!(sums.len(), 2);
    }

    #[test]
    fn short_or_non_hex_hashes_are_rejected() {
        assert!(parse_sums("abc  file\n").is_empty());
        assert!(parse_sums(&format!("{}  file\n", "z".repeat(64))).is_empty());
    }

    #[test]
    fn download_urls_must_be_this_repositorys_release_files() {
        let ok = "https://github.com/denizguzel/relink/releases/download/v0.3.0/relink-0.3.0-linux-x86_64";
        assert!(is_download_url(ok));
        for bad in [
            "https://github.com/denizguzel/relink/releases/download/v0.3.0/a/b",
            "https://github.com/other/relink/releases/download/v0.3.0/x",
            "http://github.com/denizguzel/relink/releases/download/v0.3.0/x",
            "https://github.com/denizguzel/relink/releases/download/v0.3.0/x y",
            "https://github.com/denizguzel/relink/releases/download/v0.3.0/x?a=b",
        ] {
            assert!(!is_download_url(bad), "{bad}");
        }
    }

    #[test]
    fn a_files_hash_is_computed() {
        let dir = std::env::temp_dir().join(format!("relink-sha-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("f");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(
            sha256_file(&file).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

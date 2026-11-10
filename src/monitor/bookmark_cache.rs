//! The company hosts found in browser bookmarks, re-read only on change.

use std::fs;

use crate::bookmarks;
use crate::config::Config;

/// Bookmark hosts, re-parsed only when a Bookmarks file changes.
#[derive(Default)]
pub(super) struct BookmarkCache {
    key: String,
    hosts: Vec<String>,
}

impl BookmarkCache {
    pub(super) fn hosts(&mut self, cfg: &Config) -> Vec<String> {
        if !cfg.use_bookmarks || cfg.bookmark_domains.is_empty() {
            return Vec::new();
        }
        let files = bookmarks::all_files();
        let mut key = cfg.bookmark_domains.join(",");
        for f in &files {
            let mtime = fs::metadata(f).and_then(|m| m.modified()).ok();
            key.push_str(&format!("|{}@{mtime:?}", f.display()));
        }
        if key != self.key {
            self.key = key;
            self.hosts = bookmarks::from_files(&files, &cfg.bookmark_domains);
        }
        self.hosts.clone()
    }
}

//! Keeping Re-Link current from the project's GitHub Releases: ask whether a
//! newer release exists, and install it in place when the user agrees.
//! Downloads are checked against the release's `SHA256SUMS`, and only files
//! of this repository's releases are ever fetched or opened.

mod asset;
mod fetch;
mod install;
mod release;
mod version;

pub use install::{WAIT_ARG, install, remove_old};
pub use release::{Available, Download, decide, is_release_url};
pub use version::is_newer;

/// The GitHub repository that publishes the releases.
pub const REPO: &str = "denizguzel/relink";

/// What the update, once found, is doing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// Found, waiting for the user.
    #[default]
    Ready,
    /// Downloading and installing; Re-Link restarts by itself when done.
    Installing,
    /// The in-app update failed (the reason); the release page is the fallback.
    Failed(String),
}

/// Asks GitHub whether a release newer than `current` exists.
pub fn check(current: &str) -> Result<Option<Available>, String> {
    decide(current, &fetch::latest_json()?)
}

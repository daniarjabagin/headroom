mod checker;
pub mod install;
mod outcome;
pub mod policy;
pub mod release;
mod requests;
pub mod version;

use std::sync::Arc;

use jiff::Timestamp;

pub use checker::{FeedError, FeedResponse, ReleaseFeed, run};
pub use install::{Install, InstallKind, Packager};
pub use outcome::{CheckOutcome, CheckStatus};
pub use release::{GithubAsset, GithubRelease, Release, ReleaseError};
pub use requests::{UpdateCheckRequests, UpdateChecks, channel};
pub use version::{Version, VersionError};

pub const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/daniarjabagin/headroom/releases/latest";

pub struct UpdateConfig {
    pub feed: Arc<dyn ReleaseFeed>,
    pub install: Install,
    pub current: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub release: Release,
    pub install: Install,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpdateCheckState {
    pub checked_at: Option<Timestamp>,
}

mod checker;
pub mod install;
pub mod policy;
pub mod release;
pub mod version;

use std::sync::Arc;

pub use checker::{FeedError, FeedResponse, ReleaseFeed, run};
pub use install::{Install, InstallKind, Packager};
pub use release::{GithubAsset, GithubRelease, Release, ReleaseError};
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

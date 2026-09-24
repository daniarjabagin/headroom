use jiff::Timestamp;
use serde::Deserialize;

use super::version::{Version, VersionError};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub published_at: Option<Timestamp>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub url: String,
    pub published_at: Timestamp,
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    #[error("the release JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the release tag is not a version: {0}")]
    Tag(#[from] VersionError),
    #[error("release {0} has no publication date")]
    Unpublished(String),
}

impl GithubRelease {
    pub fn parse(json: &str) -> Result<GithubRelease, ReleaseError> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn stable(&self) -> Result<Option<Release>, ReleaseError> {
        if self.draft || self.prerelease {
            return Ok(None);
        }
        let version: Version = self.tag_name.parse()?;
        if version.is_prerelease() {
            return Ok(None);
        }
        let published_at = self
            .published_at
            .ok_or_else(|| ReleaseError::Unpublished(self.tag_name.clone()))?;
        Ok(Some(Release {
            version,
            url: self.html_url.clone(),
            published_at,
        }))
    }

    #[must_use]
    pub fn asset(&self, name: &str) -> Option<&GithubAsset> {
        self.assets.iter().find(|asset| asset.name == name)
    }
}

#[must_use]
pub fn newer_than(current: &Version, latest: Option<&Release>) -> Option<Release> {
    latest.filter(|release| release.version > *current).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("fixtures/release_latest.json");

    fn fixture() -> GithubRelease {
        GithubRelease::parse(FIXTURE).unwrap()
    }

    #[test]
    fn the_latest_release_maps_to_a_stable_release() {
        let release = fixture().stable().unwrap().unwrap();
        assert_eq!(release.version, "0.5.0".parse().unwrap());
        assert_eq!(
            release.url,
            "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0"
        );
        assert_eq!(
            release.published_at,
            "2026-10-01T09:20:02Z".parse::<Timestamp>().unwrap()
        );
    }

    #[test]
    fn assets_are_found_by_name() {
        let raw = fixture();
        let tarball = raw
            .asset("headroom-0.5.0-x86_64-linux-musl.tar.gz")
            .unwrap();
        assert!(
            tarball
                .browser_download_url
                .ends_with("/v0.5.0/headroom-0.5.0-x86_64-linux-musl.tar.gz")
        );
        assert!(raw.asset("SHA256SUMS").is_some());
        assert!(
            raw.asset("headroom-0.5.0-riscv64-linux-musl.tar.gz")
                .is_none()
        );
    }

    #[test]
    fn drafts_and_prereleases_are_ignored() {
        let draft = GithubRelease {
            draft: true,
            ..fixture()
        };
        let flagged = GithubRelease {
            prerelease: true,
            ..fixture()
        };
        let tagged = GithubRelease {
            tag_name: "v0.6.0-rc.1".into(),
            ..fixture()
        };
        for raw in [draft, flagged, tagged] {
            assert_eq!(raw.stable().unwrap(), None);
        }
    }

    #[test]
    fn a_bad_tag_or_missing_date_is_an_error() {
        let tag = GithubRelease {
            tag_name: "nightly".into(),
            ..fixture()
        };
        assert!(matches!(tag.stable(), Err(ReleaseError::Tag(_))));
        let undated = GithubRelease {
            published_at: None,
            ..fixture()
        };
        assert!(matches!(
            undated.stable(),
            Err(ReleaseError::Unpublished(_))
        ));
        assert!(matches!(
            GithubRelease::parse(r#"{"tag_name":"v1.0.0"}"#),
            Err(ReleaseError::Json(_))
        ));
    }

    #[test]
    fn only_a_higher_version_is_an_update() {
        let latest = fixture().stable().unwrap();
        let older: Version = "0.4.0".parse().unwrap();
        let same: Version = "0.5.0".parse().unwrap();
        let candidate: Version = "0.5.0-rc.2".parse().unwrap();
        assert_eq!(newer_than(&older, latest.as_ref()), latest);
        assert_eq!(newer_than(&candidate, latest.as_ref()), latest);
        assert_eq!(newer_than(&same, latest.as_ref()), None);
        assert_eq!(newer_than(&older, None), None);
    }
}

use jiff::Timestamp;
use serde::Serialize;

use super::release::newer_than;
use super::version::Version;
use crate::storage::updates::CheckRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    UpToDate,
    Available,
    Failed,
    RateLimited,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckOutcome {
    pub status: CheckStatus,
    pub checked_at: Option<Timestamp>,
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<Timestamp>,
}

impl CheckOutcome {
    #[must_use]
    pub fn disabled() -> CheckOutcome {
        CheckOutcome {
            status: CheckStatus::Disabled,
            checked_at: None,
            version: None,
            until: None,
        }
    }

    #[must_use]
    pub fn checked(current: &Version, record: Option<&CheckRecord>) -> CheckOutcome {
        let latest = record.and_then(|record| record.latest.as_ref());
        let status = match newer_than(current, latest) {
            Some(_) => CheckStatus::Available,
            None => CheckStatus::UpToDate,
        };
        CheckOutcome::known(status, record)
    }

    #[must_use]
    pub fn failed(record: Option<&CheckRecord>) -> CheckOutcome {
        CheckOutcome::known(CheckStatus::Failed, record)
    }

    #[must_use]
    pub fn rate_limited(record: Option<&CheckRecord>, until: Timestamp) -> CheckOutcome {
        CheckOutcome {
            until: Some(until),
            ..CheckOutcome::known(CheckStatus::RateLimited, record)
        }
    }

    fn known(status: CheckStatus, record: Option<&CheckRecord>) -> CheckOutcome {
        CheckOutcome {
            status,
            checked_at: record.map(|record| record.checked_at),
            version: record
                .and_then(|record| record.latest.as_ref())
                .map(|release| release.version.to_string()),
            until: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::update::Release;

    fn record(version: &str) -> CheckRecord {
        CheckRecord {
            checked_at: "2026-09-23T10:00:00Z".parse().unwrap(),
            etag: None,
            latest: Some(Release {
                version: version.parse().unwrap(),
                url: "https://example.invalid".into(),
                published_at: "2026-09-20T10:00:00Z".parse().unwrap(),
            }),
        }
    }

    fn current() -> Version {
        "0.5.0".parse().unwrap()
    }

    fn json_of(outcome: &CheckOutcome) -> serde_json::Value {
        serde_json::to_value(outcome).unwrap()
    }

    #[test]
    fn a_newer_release_is_available_and_the_same_one_is_up_to_date() {
        let newer = record("0.6.0");
        assert_eq!(
            json_of(&CheckOutcome::checked(&current(), Some(&newer))),
            json!({"status": "available", "checked_at": "2026-09-23T10:00:00Z", "version": "0.6.0"})
        );
        let same = record("0.5.0");
        assert_eq!(
            json_of(&CheckOutcome::checked(&current(), Some(&same))),
            json!({"status": "up_to_date", "checked_at": "2026-09-23T10:00:00Z", "version": "0.5.0"})
        );
    }

    #[test]
    fn failures_keep_the_last_successful_check() {
        assert_eq!(
            json_of(&CheckOutcome::failed(Some(&record("0.6.0")))),
            json!({"status": "failed", "checked_at": "2026-09-23T10:00:00Z", "version": "0.6.0"})
        );
        assert_eq!(
            json_of(&CheckOutcome::failed(None)),
            json!({"status": "failed", "checked_at": null, "version": null})
        );
    }

    #[test]
    fn only_rate_limits_carry_until() {
        let until = "2026-09-23T11:30:00Z".parse().unwrap();
        assert_eq!(
            json_of(&CheckOutcome::rate_limited(None, until)),
            json!({
                "status": "rate_limited",
                "checked_at": null,
                "version": null,
                "until": "2026-09-23T11:30:00Z"
            })
        );
        assert_eq!(
            json_of(&CheckOutcome::disabled()),
            json!({"status": "disabled", "checked_at": null, "version": null})
        );
    }
}

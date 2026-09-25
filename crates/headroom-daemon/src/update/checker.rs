use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use jiff::{SignedDuration, Timestamp};
use tokio::time::{Instant, sleep_until};

use super::outcome::CheckOutcome;
use super::release::{GithubRelease, ReleaseError, newer_than};
use super::requests::{self, UpdateCheckRequests};
use super::{AvailableUpdate, UpdateCheckState, UpdateConfig, policy};
use crate::core::Core;
use crate::storage::updates::{self, CheckRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedResponse {
    Modified { body: String, etag: Option<String> },
    NotModified,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedError {
    #[error("GitHub's rate limit was reached")]
    RateLimited { retry_after: Option<SignedDuration> },
    #[error("{0}")]
    Failed(String),
}

#[async_trait]
pub trait ReleaseFeed: Send + Sync {
    async fn latest(&self, etag: Option<&str>) -> Result<FeedResponse, FeedError>;
}

struct Checker {
    core: Arc<Core>,
    config: UpdateConfig,
    record: Option<CheckRecord>,
    hold_until: Option<Timestamp>,
    last: Option<Attempt>,
}

struct Attempt {
    at: Timestamp,
    outcome: CheckOutcome,
}

pub async fn run(core: Arc<Core>, config: UpdateConfig, mut requests: UpdateCheckRequests) {
    let mut checker = Checker::load(core, config).await;
    let mut settings = checker.core.settings_changes();
    let mut deadline = Instant::now() + to_std(checker.first_delay());
    loop {
        let enabled = checker.enabled();
        tokio::select! {
            () = sleep_until(deadline), if enabled => {
                let (_, delay) = checker.check().await;
                deadline = Instant::now() + to_std(delay);
            }
            changed = settings.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            Some(waiters) = requests.next() => {
                let outcome = checker.manual(&mut deadline).await;
                requests::answer(waiters, &outcome);
            }
        }
    }
}

impl Checker {
    async fn load(core: Arc<Core>, config: UpdateConfig) -> Checker {
        let record = match core.storage.run(|conn| updates::load(conn)).await {
            Ok(record) => record,
            Err(error) => {
                tracing::warn!(%error, "could not load the last update check");
                None
            }
        };
        let checker = Checker {
            core,
            config,
            record,
            hold_until: None,
            last: None,
        };
        checker.publish();
        checker
    }

    fn enabled(&self) -> bool {
        self.core.model().settings.updates.check
    }

    fn first_delay(&self) -> SignedDuration {
        let last = self.record.as_ref().map(|record| record.checked_at);
        policy::first_delay(last, self.core.clock.now(), self.core.random.unit())
    }

    async fn manual(&mut self, deadline: &mut Instant) -> CheckOutcome {
        if !self.enabled() {
            return CheckOutcome::disabled();
        }
        let now = self.core.clock.now();
        if let Some(until) = self.hold_until.filter(|until| now < *until) {
            return CheckOutcome::rate_limited(self.record.as_ref(), until);
        }
        if let Some(last) = &self.last
            && policy::reusable(last.at, now)
        {
            return last.outcome.clone();
        }
        let (outcome, delay) = self.check().await;
        *deadline = Instant::now() + to_std(delay);
        outcome
    }

    async fn check(&mut self) -> (CheckOutcome, SignedDuration) {
        let at = self.core.clock.now();
        let (outcome, delay) = self.attempt(at).await;
        self.last = Some(Attempt {
            at,
            outcome: outcome.clone(),
        });
        (outcome, delay)
    }

    async fn attempt(&mut self, now: Timestamp) -> (CheckOutcome, SignedDuration) {
        let etag = self.record.as_ref().and_then(|record| record.etag.clone());
        let sample = self.core.random.unit();
        let response = match self.config.feed.latest(etag.as_deref()).await {
            Ok(response) => response,
            Err(FeedError::RateLimited { retry_after }) => {
                tracing::debug!("update check rate limited by GitHub");
                return self.hold(now, policy::after_rate_limit(retry_after));
            }
            Err(FeedError::Failed(error)) => {
                tracing::debug!(%error, "update check failed");
                return (self.failed(), policy::after_failure(sample));
            }
        };
        match accept(self.record.as_ref(), response, now) {
            Ok(record) => {
                self.store(record).await;
                let outcome = CheckOutcome::checked(&self.config.current, self.record.as_ref());
                (outcome, policy::after_check(sample))
            }
            Err(error) => {
                tracing::debug!(%error, "update check returned an unusable release");
                (self.failed(), policy::after_failure(sample))
            }
        }
    }

    fn hold(&mut self, now: Timestamp, delay: SignedDuration) -> (CheckOutcome, SignedDuration) {
        let until = now.checked_add(delay).unwrap_or(Timestamp::MAX);
        self.hold_until = Some(until);
        (
            CheckOutcome::rate_limited(self.record.as_ref(), until),
            delay,
        )
    }

    fn failed(&self) -> CheckOutcome {
        CheckOutcome::failed(self.record.as_ref())
    }

    async fn store(&mut self, record: CheckRecord) {
        let stored = record.clone();
        let saved = self
            .core
            .storage
            .run(move |conn| updates::save(conn, &stored))
            .await;
        if let Err(error) = saved {
            tracing::warn!(%error, "could not store the update check");
        }
        self.record = Some(record);
        self.publish();
    }

    fn publish(&self) {
        let latest = self
            .record
            .as_ref()
            .and_then(|record| record.latest.as_ref());
        let update = newer_than(&self.config.current, latest).map(|release| AvailableUpdate {
            release,
            install: self.config.install,
        });
        let check = Some(UpdateCheckState {
            checked_at: self.record.as_ref().map(|record| record.checked_at),
        });
        let mut model = self.core.model();
        if model.update == update && model.update_check == check {
            return;
        }
        model.update = update;
        model.update_check = check;
        drop(model);
        self.core.mark_changed();
    }
}

fn accept(
    previous: Option<&CheckRecord>,
    response: FeedResponse,
    now: Timestamp,
) -> Result<CheckRecord, ReleaseError> {
    match response {
        FeedResponse::NotModified => Ok(CheckRecord {
            checked_at: now,
            etag: previous.and_then(|record| record.etag.clone()),
            latest: previous.and_then(|record| record.latest.clone()),
        }),
        FeedResponse::Modified { body, etag } => Ok(CheckRecord {
            checked_at: now,
            etag,
            latest: GithubRelease::parse(&body)?.stable()?,
        }),
    }
}

fn to_std(delay: SignedDuration) -> Duration {
    Duration::try_from(delay).unwrap_or_default()
}

#[cfg(test)]
#[path = "test_feed.rs"]
mod test_feed;

#[cfg(test)]
#[path = "checker_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "manual_tests.rs"]
mod manual_tests;

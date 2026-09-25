use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use headroom_core::event::UsageEvent;
use headroom_core::units::MicroUsd;
use headroom_core::usage::PriceBook;
use headroom_pricing::{FeedStatus, PriceCatalog, RefreshOutcome, Sources};

const REFRESH_EVERY: Duration = Duration::from_hours(24);
const RETRY_AFTER: Duration = Duration::from_mins(30);

pub struct ReloadablePrices {
    dir: PathBuf,
    catalog: RwLock<Arc<PriceCatalog>>,
}

impl ReloadablePrices {
    pub fn load(dir: PathBuf) -> Result<ReloadablePrices> {
        let catalog = PriceCatalog::load(&dir).context("could not load the price catalog")?;
        Ok(ReloadablePrices {
            dir,
            catalog: RwLock::new(Arc::new(catalog)),
        })
    }

    fn current(&self) -> Arc<PriceCatalog> {
        self.catalog
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn reload(&self) {
        match PriceCatalog::load(&self.dir) {
            Ok(catalog) => {
                *self.catalog.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(catalog);
                tracing::info!("price catalog reloaded");
            }
            Err(error) => tracing::warn!(%error, "could not reload the price catalog"),
        }
    }
}

impl PriceBook for ReloadablePrices {
    fn cost(&self, event: &UsageEvent) -> Option<MicroUsd> {
        self.current().cost(event)
    }
}

pub async fn keep_fresh(prices: Arc<ReloadablePrices>, client: reqwest::Client) {
    let sources = Sources::default();
    loop {
        let delay = match headroom_pricing::refresh(&prices.dir, &client, &sources).await {
            Ok(outcome) => {
                if any_updated(&outcome) {
                    prices.reload();
                }
                next_delay(&outcome)
            }
            Err(error) => {
                tracing::warn!(%error, "price refresh failed");
                RETRY_AFTER
            }
        };
        tokio::time::sleep(delay).await;
    }
}

fn feeds(outcome: &RefreshOutcome) -> [&FeedStatus; 2] {
    [&outcome.litellm, &outcome.models_dev]
}

fn any_updated(outcome: &RefreshOutcome) -> bool {
    feeds(outcome)
        .iter()
        .any(|status| matches!(status, FeedStatus::Updated))
}

fn next_delay(outcome: &RefreshOutcome) -> Duration {
    let failed = feeds(outcome)
        .iter()
        .any(|status| matches!(status, FeedStatus::Failed(_)));
    if failed { RETRY_AFTER } else { REFRESH_EVERY }
}

#[cfg(test)]
mod tests {
    use headroom_core::event::{EventKey, ServiceTier};
    use headroom_core::tokens::TokenCounts;
    use headroom_core::units::Tokens;
    use headroom_pricing::PricingError;

    use super::*;

    fn failed() -> FeedStatus {
        FeedStatus::Failed(PricingError::Status {
            url: "https://example.test".into(),
            status: 503,
        })
    }

    fn outcome(litellm: FeedStatus, models_dev: FeedStatus) -> RefreshOutcome {
        RefreshOutcome {
            litellm,
            models_dev,
        }
    }

    #[test]
    fn a_failed_feed_retries_in_thirty_minutes() {
        let mixed = outcome(FeedStatus::Updated, failed());
        assert_eq!(next_delay(&mixed), RETRY_AFTER);
        assert!(any_updated(&mixed));
    }

    #[test]
    fn healthy_feeds_wait_a_day() {
        let unchanged = outcome(FeedStatus::NotModified, FeedStatus::NotModified);
        assert_eq!(next_delay(&unchanged), REFRESH_EVERY);
        assert!(!any_updated(&unchanged));
    }

    #[test]
    fn prices_from_an_empty_cache_fall_back_to_the_bundled_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let prices = ReloadablePrices::load(dir.path().to_path_buf()).unwrap();
        let event = UsageEvent {
            key: EventKey("resp_1".into()),
            at: "2026-09-23T10:00:00Z".parse().unwrap(),
            model: "codex-auto-review".into(),
            tier: ServiceTier::Standard,
            tokens: TokenCounts {
                input: Tokens(1_000_000),
                ..TokenCounts::default()
            },
            web_search_requests: 0,
            reported_cost: None,
            project: None,
        };
        let cost = prices.cost(&event);
        assert!(cost.is_some_and(|micros| micros.0 > 0));
        prices.reload();
        assert_eq!(prices.cost(&event), cost);
    }
}

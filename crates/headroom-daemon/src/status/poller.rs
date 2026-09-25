use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use jiff::SignedDuration;
use tokio::time::{Instant, sleep_until};

use super::assess::assess;
use super::fetch::{FetchError, Fetched, StatusFetch};
use super::sources::{self, StatusSource};
use super::{ProviderStatus, policy, providers_in_use};
use crate::core::Core;
use crate::storage::status_cache::{self, CachedPage};

struct Poller {
    core: Arc<Core>,
    fetch: Arc<dyn StatusFetch>,
    pages: HashMap<String, CachedPage>,
    holds: HashMap<String, Hold>,
}

struct Hold {
    failures: u32,
    until: Instant,
}

pub async fn run(core: Arc<Core>, fetch: Arc<dyn StatusFetch>) {
    let mut poller = Poller::load(core, fetch).await;
    let mut settings = poller.core.settings_changes();
    let mut enabled = poller.enabled();
    let mut deadline = Instant::now() + to_std(poller.first_delay());
    loop {
        tokio::select! {
            () = sleep_until(deadline), if enabled => {
                poller.round().await;
                let delay = policy::next_round(poller.core.random.unit());
                deadline = Instant::now() + to_std(delay);
            }
            changed = settings.changed() => {
                if changed.is_err() {
                    return;
                }
                let was_enabled = std::mem::replace(&mut enabled, poller.enabled());
                if enabled && !was_enabled {
                    deadline = Instant::now();
                }
            }
        }
    }
}

impl Poller {
    async fn load(core: Arc<Core>, fetch: Arc<dyn StatusFetch>) -> Poller {
        let cached = match core.storage.run(|conn| status_cache::load_all(conn)).await {
            Ok(cached) => cached,
            Err(error) => {
                tracing::warn!(%error, "could not load the cached status pages");
                Vec::new()
            }
        };
        let poller = Poller {
            core,
            fetch,
            pages: cached
                .into_iter()
                .map(|page| (page.url.clone(), page))
                .collect(),
            holds: HashMap::new(),
        };
        poller.publish();
        poller
    }

    fn enabled(&self) -> bool {
        self.core.model().settings.status_pages.enabled
    }

    fn page_of(&self, source: &StatusSource) -> Option<&'static str> {
        self.core.catalog.descriptor(&source.provider)?.links.status
    }

    fn wanted(&self) -> Vec<String> {
        let in_use = providers_in_use(&self.core.model());
        let mut urls: Vec<String> = Vec::new();
        for source in sources::all().filter(|source| in_use.contains(&source.provider)) {
            let Some(page) = self.page_of(source) else {
                continue;
            };
            for url in source.endpoints(page).urls() {
                if !urls.iter().any(|known| known == url) {
                    urls.push(url.to_owned());
                }
            }
        }
        urls
    }

    fn first_delay(&self) -> SignedDuration {
        let fetched: Vec<_> = self
            .wanted()
            .iter()
            .map(|url| self.pages.get(url).map(|page| page.fetched_at))
            .collect();
        policy::first_round(&fetched, self.core.clock.now())
    }

    async fn round(&mut self) {
        let started = Instant::now();
        for url in self.wanted() {
            let held = self
                .holds
                .get(&url)
                .is_some_and(|hold| hold.until > started);
            if !held {
                self.poll(url, started).await;
            }
        }
        self.publish();
    }

    async fn poll(&mut self, url: String, started: Instant) {
        let etag = self.pages.get(&url).and_then(|page| page.etag.clone());
        let now = self.core.clock.now();
        let fetched = match self.fetch.get(&url, etag.as_deref()).await {
            Ok(Fetched::Modified { body, etag }) => Some(CachedPage {
                url: url.clone(),
                fetched_at: now,
                etag,
                body,
            }),
            Ok(Fetched::NotModified) => self.pages.get(&url).map(|page| CachedPage {
                fetched_at: now,
                ..page.clone()
            }),
            Err(FetchError::RateLimited { retry_after }) => {
                tracing::debug!(%url, "status page asked Headroom to slow down");
                self.hold(url, started, policy::after_rate_limit(retry_after));
                return;
            }
            Err(FetchError::Failed(error)) => {
                self.fail(url, started, &error);
                return;
            }
        };
        match fetched {
            Some(page) => self.store(page).await,
            None => self.fail(url, started, "answered 304 to a page Headroom never read"),
        }
    }

    fn fail(&mut self, url: String, started: Instant, error: &str) {
        tracing::debug!(%url, %error, "status page request failed");
        let failures = self.failures(&url).saturating_add(1);
        let wait = policy::after_failure(failures, self.core.random.unit());
        self.hold(url, started, wait);
    }

    fn hold(&mut self, url: String, started: Instant, wait: SignedDuration) {
        let failures = self.failures(&url).saturating_add(1);
        let until = started + to_std(wait);
        self.holds.insert(url, Hold { failures, until });
    }

    fn failures(&self, url: &str) -> u32 {
        self.holds.get(url).map_or(0, |hold| hold.failures)
    }

    async fn store(&mut self, page: CachedPage) {
        self.holds.remove(&page.url);
        let stored = page.clone();
        let saved = self
            .core
            .storage
            .run(move |conn| status_cache::save(conn, &stored))
            .await;
        if let Err(error) = saved {
            tracing::warn!(%error, "could not cache a status page");
        }
        self.pages.insert(page.url.clone(), page);
    }

    fn publish(&self) {
        let statuses: BTreeMap<_, _> = sources::all()
            .filter_map(|source| Some((source.provider.clone(), self.status_of(source)?)))
            .collect();
        let mut model = self.core.model();
        if model.provider_status == statuses {
            return;
        }
        model.provider_status = statuses;
        drop(model);
        self.core.mark_changed();
    }

    fn status_of(&self, source: &StatusSource) -> Option<ProviderStatus> {
        let page = self.page_of(source)?;
        let fetched_at = source
            .endpoints(page)
            .urls()
            .map(|url| self.pages.get(url).map(|page| page.fetched_at))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .min()?;
        let body = |url: &str| self.pages.get(url).map(|page| page.body.as_str());
        match assess(source, page, body) {
            Ok(assessment) => Some(ProviderStatus {
                assessment,
                fetched_at,
            }),
            Err(error) => {
                tracing::debug!(provider = %source.provider, %error, "status page unreadable");
                None
            }
        }
    }
}

fn to_std(delay: SignedDuration) -> Duration {
    Duration::try_from(delay).unwrap_or_default()
}

#[cfg(test)]
#[path = "test_fetch.rs"]
mod test_fetch;

#[cfg(test)]
#[path = "poller_tests.rs"]
mod tests;

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;
use headroom_core::account::ProviderId;
use headroom_core::provider::Provider;
use tokio::task::JoinHandle;

use super::*;
use crate::testing::{FakeProvider, Harness, account, harness, snapshot};

pub const NOW: &str = "2026-09-23T10:00:00Z";
pub const CLAUDE_URL: &str = "https://status.claude.com/api/v2/summary.json";
pub const OPENAI_COMPONENTS_URL: &str = "https://status.openai.com/api/v2/components.json";
pub const OPENAI_WIDGET_URL: &str = "https://status.openai.com/api/v1/summary";
pub const CLAUDE_OUTAGE: &str = include_str!("fixtures/statuspage_claude_outage_constructed.json");
pub const OPENAI_DEGRADED: &str =
    include_str!("fixtures/incidentio_openai_components_degraded_constructed.json");
pub const OPENAI_WIDGET: &str =
    include_str!("fixtures/incidentio_openai_widget_ongoing_constructed.json");

pub type Answer = Result<Fetched, FetchError>;

pub struct Call {
    pub at: Instant,
    pub url: String,
    pub etag: Option<String>,
}

#[derive(Default)]
pub struct FakeFetch {
    script: Mutex<HashMap<String, VecDeque<Answer>>>,
    calls: Mutex<Vec<Call>>,
}

impl FakeFetch {
    pub fn new(script: Vec<(&str, Vec<Answer>)>) -> Arc<FakeFetch> {
        let script = script
            .into_iter()
            .map(|(url, answers)| (url.to_owned(), answers.into()))
            .collect();
        Arc::new(FakeFetch {
            script: Mutex::new(script),
            calls: Mutex::default(),
        })
    }

    pub fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn urls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|call| call.url.clone())
            .collect()
    }

    pub fn etags(&self) -> Vec<Option<String>> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|call| call.etag.clone())
            .collect()
    }

    pub fn gaps_from(&self, start: Instant) -> Vec<u64> {
        let calls = self.calls.lock().unwrap();
        let mut previous = start;
        calls
            .iter()
            .map(|call| {
                let gap = (call.at - previous).as_secs();
                previous = call.at;
                gap
            })
            .collect()
    }
}

#[async_trait]
impl StatusFetch for FakeFetch {
    async fn get(&self, url: &str, etag: Option<&str>) -> Result<Fetched, FetchError> {
        self.calls.lock().unwrap().push(Call {
            at: Instant::now(),
            url: url.to_owned(),
            etag: etag.map(str::to_owned),
        });
        let next = self
            .script
            .lock()
            .unwrap()
            .get_mut(url)
            .and_then(VecDeque::pop_front);
        next.unwrap_or_else(|| Err(FetchError::Failed("script exhausted".into())))
    }
}

pub fn modified(body: &str, etag: Option<&str>) -> Fetched {
    Fetched::Modified {
        body: body.into(),
        etag: etag.map(str::to_owned),
    }
}

pub async fn harness_using(providers: &[ProviderId]) -> Harness {
    let providers = providers
        .iter()
        .map(|provider| {
            let accounts = vec![account(provider.clone(), "main")];
            let provider = FakeProvider::new(provider.clone(), accounts, snapshot(Vec::new(), NOW));
            Arc::new(provider) as Arc<dyn Provider>
        })
        .collect();
    harness(providers).await
}

pub fn set_enabled(harness: &Harness, enabled: bool) {
    harness.core.model().settings.status_pages.enabled = enabled;
    harness.core.settings_stored();
}

pub fn start(harness: &Harness, fetch: Arc<FakeFetch>) -> JoinHandle<()> {
    tokio::spawn(run(harness.core.clone(), fetch))
}

pub async fn until_calls(fetch: &FakeFetch, count: usize) {
    for _ in 0..24 * 60 {
        if fetch.count() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    panic!(
        "status pages were requested {} times, not {count}",
        fetch.count()
    );
}

pub fn cached(harness: &Harness) -> Vec<CachedPage> {
    harness
        .storage
        .blocking(|conn| status_cache::load_all(conn))
        .unwrap()
}

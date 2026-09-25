use std::collections::VecDeque;
use std::sync::Mutex;

use tokio::task::JoinHandle;

use super::*;
use crate::testing::Harness;
use crate::update::{Install, Release, UpdateChecks, Version, channel};

pub const FIXTURE: &str = include_str!("fixtures/release_latest.json");
pub const NOW: &str = "2026-09-23T10:00:00Z";

type Answer = Result<FeedResponse, FeedError>;

#[derive(Default)]
pub struct FakeFeed {
    script: Mutex<VecDeque<Answer>>,
    calls: Mutex<Vec<(Instant, Option<String>)>>,
}

impl FakeFeed {
    pub fn new(script: Vec<Answer>) -> Arc<FakeFeed> {
        Arc::new(FakeFeed {
            script: Mutex::new(script.into()),
            calls: Mutex::default(),
        })
    }

    pub fn calls(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn etags(&self) -> Vec<Option<String>> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.1.clone())
            .collect()
    }

    pub fn gaps_from(&self, start: Instant) -> Vec<u64> {
        let calls = self.calls.lock().unwrap();
        let mut previous = start;
        calls
            .iter()
            .map(|(at, _)| {
                let gap = (*at - previous).as_secs();
                previous = *at;
                gap
            })
            .collect()
    }
}

#[async_trait]
impl ReleaseFeed for FakeFeed {
    async fn latest(&self, etag: Option<&str>) -> Result<FeedResponse, FeedError> {
        self.calls
            .lock()
            .unwrap()
            .push((Instant::now(), etag.map(str::to_owned)));
        let next = self.script.lock().unwrap().pop_front();
        next.unwrap_or_else(|| Err(FeedError::Failed("script exhausted".into())))
    }
}

pub async fn until_calls(feed: &FakeFeed, count: usize) {
    for _ in 0..5 * 24 * 60 {
        if feed.calls() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
    panic!("the feed was called {} times, not {count}", feed.calls());
}

pub fn modified(etag: &str) -> FeedResponse {
    FeedResponse::Modified {
        body: FIXTURE.into(),
        etag: Some(etag.into()),
    }
}

pub fn release() -> Release {
    GithubRelease::parse(FIXTURE)
        .unwrap()
        .stable()
        .unwrap()
        .unwrap()
}

pub struct Running {
    task: JoinHandle<()>,
    pub checks: UpdateChecks,
}

impl Running {
    pub fn abort(&self) {
        self.task.abort();
    }
}

pub fn start(harness: &Harness, feed: Arc<FakeFeed>, current: &str) -> Running {
    let config = UpdateConfig {
        feed,
        install: Install::Script,
        current: current.parse::<Version>().unwrap(),
    };
    let (checks, requests) = channel();
    let task = tokio::spawn(run(harness.core.clone(), config, requests));
    Running { task, checks }
}

pub fn stored(harness: &Harness) -> Option<CheckRecord> {
    harness
        .storage
        .blocking(|conn| updates::load(conn))
        .unwrap()
}

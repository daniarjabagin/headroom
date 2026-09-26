use std::collections::{BTreeMap, HashMap};

use headroom_core::account::AccountId;
use headroom_core::history::{SampleStep, UsageSample, is_retained, observed_at, sample_step};
use headroom_core::quota::{LimitsSnapshot, QuotaWindow};
use jiff::Timestamp;

use crate::model::window_key;
use crate::storage::samples::SampleRow;

pub type WindowSamples = BTreeMap<String, Vec<UsageSample>>;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuotaHistory {
    accounts: HashMap<AccountId, WindowSamples>,
}

impl QuotaHistory {
    #[must_use]
    pub fn from_rows(rows: Vec<SampleRow>) -> QuotaHistory {
        let mut history = QuotaHistory::default();
        for (id, key, sample) in rows {
            history
                .accounts
                .entry(id)
                .or_default()
                .entry(key)
                .or_default()
                .push(sample);
        }
        history
    }

    pub fn record(&mut self, id: &AccountId, snapshot: &LimitsSnapshot, now: Timestamp) {
        let at = observed_at(snapshot);
        let windows = self.accounts.entry(id.clone()).or_default();
        for window in &snapshot.windows {
            let samples = windows.entry(window_key(&window.id)).or_default();
            record_window(samples, window, at);
        }
        self.prune(now);
    }

    #[must_use]
    pub fn account(&self, id: &AccountId) -> Option<&WindowSamples> {
        self.accounts.get(id)
    }

    fn prune(&mut self, now: Timestamp) {
        for windows in self.accounts.values_mut() {
            for samples in windows.values_mut() {
                samples.retain(|sample| is_retained(sample, now));
            }
            windows.retain(|_, samples| !samples.is_empty());
        }
        self.accounts.retain(|_, windows| !windows.is_empty());
    }
}

#[must_use]
pub fn window_samples<'a>(windows: Option<&'a WindowSamples>, key: &str) -> &'a [UsageSample] {
    windows
        .and_then(|windows| windows.get(key))
        .map_or(&[], Vec::as_slice)
}

fn record_window(samples: &mut Vec<UsageSample>, window: &QuotaWindow, at: Timestamp) {
    let sample = UsageSample {
        at,
        used: window.used,
    };
    match sample_step(samples.last(), window, at) {
        Some(SampleStep::Restart) => {
            samples.clear();
            samples.push(sample);
        }
        Some(SampleStep::Append) => samples.push(sample),
        None => {}
    }
}

#[cfg(test)]
#[path = "quota_history_tests.rs"]
mod tests;

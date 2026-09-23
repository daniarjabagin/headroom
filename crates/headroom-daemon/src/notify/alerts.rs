use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use headroom_core::account::AccountId;
use headroom_core::quota::{LimitsSnapshot, QuotaWindow};
use jiff::Timestamp;
use rusqlite::Connection;

use super::Notifier;
use super::evaluator::{AlertState, Evaluation, Milestone, Observation, evaluate, rollback};
use super::text::{Subject, compose};
use crate::error::StorageError;
use crate::model::window_key;
use crate::settings::NotificationSettings;
use crate::storage::Storage;
use crate::storage::accounts::AccountRecord;
use crate::storage::alerts;

type AlertKey = (AccountId, String);

pub struct Alerts {
    states: Mutex<HashMap<AlertKey, AlertState>>,
    notifier: Arc<dyn Notifier>,
    storage: Storage,
}

pub struct Review<'a> {
    pub account: &'a AccountRecord,
    pub snapshot: &'a LimitsSnapshot,
    pub settings: NotificationSettings,
    pub now: Timestamp,
}

impl Alerts {
    pub fn load(
        conn: &Connection,
        storage: Storage,
        notifier: Arc<dyn Notifier>,
    ) -> Result<Alerts, StorageError> {
        let states = alerts::load_all(conn)?
            .into_iter()
            .map(|(account, window, state)| ((account, window), state))
            .collect();
        Ok(Alerts {
            states: Mutex::new(states),
            notifier,
            storage,
        })
    }

    pub async fn review(&self, review: &Review<'_>) -> Result<(), StorageError> {
        if !review.account.is_visible() {
            return Ok(());
        }
        for window in &review.snapshot.windows {
            self.review_window(review, window).await?;
        }
        Ok(())
    }

    async fn review_window(
        &self,
        review: &Review<'_>,
        window: &QuotaWindow,
    ) -> Result<(), StorageError> {
        let key = (review.account.id().clone(), window_key(&window.id));
        let observed = Observation::of(window, review.now);
        let previous = self.state(&key);
        let mut evaluation = evaluate(previous.as_ref(), &observed);
        self.deliver(review, window, &observed, &mut evaluation)
            .await;
        if previous.as_ref() != Some(&evaluation.state) {
            self.persist(&key, &evaluation.state).await?;
        }
        self.states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, evaluation.state);
        Ok(())
    }

    async fn deliver(
        &self,
        review: &Review<'_>,
        window: &QuotaWindow,
        observed: &Observation,
        evaluation: &mut Evaluation,
    ) {
        let account = review.account;
        let subject = Subject {
            provider: account.reference.provider,
            account_name: account.label.as_deref().or(account.email.as_deref()),
            window_label: &window.label,
        };
        for milestone in evaluation.alerts.clone() {
            if !enabled(review.settings, milestone) {
                continue;
            }
            let notification = compose(milestone, &subject, observed, review.now);
            if let Err(error) = self.notifier.notify(&notification).await {
                tracing::warn!(%error, ?milestone, "notification not delivered, will retry");
                rollback(&mut evaluation.state, milestone);
            }
        }
    }

    fn state(&self, key: &AlertKey) -> Option<AlertState> {
        self.states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
    }

    async fn persist(&self, key: &AlertKey, state: &AlertState) -> Result<(), StorageError> {
        let (account, window) = key.clone();
        let state = state.clone();
        self.storage
            .run(move |conn| alerts::save(conn, &account, &window, &state))
            .await
    }
}

fn enabled(settings: NotificationSettings, milestone: Milestone) -> bool {
    match milestone {
        Milestone::AlmostOut => settings.almost_out,
        Milestone::CuttingItClose => settings.cutting_it_close,
        Milestone::WillRunOut => settings.will_run_out,
        Milestone::Reset => settings.reset,
    }
}

#[cfg(test)]
#[path = "alerts_tests.rs"]
mod tests;

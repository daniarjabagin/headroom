use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use headroom_core::account::AccountId;
use headroom_core::quota::{LimitsSnapshot, QuotaWindow};
use jiff::Timestamp;
use rusqlite::Connection;

use super::Notifier;
use super::evaluator::{AlertState, Evaluation, Milestone, Observation, evaluate, rollback};
use super::text::{Locale, Subject, compose, compose_lapse};
use crate::error::StorageError;
use crate::model::window_key;
use crate::settings::{DisplaySettings, NotificationSettings};
use crate::storage::Storage;
use crate::storage::accounts::AccountRecord;
use crate::storage::{alerts, lapses};

type AlertKey = (AccountId, String);

pub struct Alerts {
    states: Mutex<HashMap<AlertKey, AlertState>>,
    lapses_notified: Mutex<HashSet<AccountId>>,
    notifier: Arc<dyn Notifier>,
    storage: Storage,
}

pub struct Review<'a> {
    pub account: &'a AccountRecord,
    pub provider_name: &'a str,
    pub snapshot: &'a LimitsSnapshot,
    pub settings: NotificationSettings,
    pub display: &'a DisplaySettings,
    pub locale: Locale,
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
        let lapses_notified = lapses::load_all(conn)?
            .into_iter()
            .filter(|lapse| lapse.notified)
            .map(|lapse| lapse.account)
            .collect();
        Ok(Alerts {
            states: Mutex::new(states),
            lapses_notified: Mutex::new(lapses_notified),
            notifier,
            storage,
        })
    }

    pub async fn review(&self, review: &Review<'_>) -> Result<(), StorageError> {
        if !review.account.is_visible() {
            return Ok(());
        }
        let id = &review.account.id().0;
        for window in &review.snapshot.windows {
            if !review.display.is_hidden(id, &window_key(&window.id)) {
                self.review_window(review, window).await?;
            }
        }
        Ok(())
    }

    pub async fn review_lapse(
        &self,
        account: &AccountRecord,
        provider_name: &str,
        locale: Locale,
    ) -> Result<(), StorageError> {
        let id = account.id().clone();
        if !account.is_visible() || self.lapse_notified().contains(&id) {
            return Ok(());
        }
        let name = account.label.as_deref().or(account.email.as_deref());
        let notification = compose_lapse(locale, &id.0, provider_name, name);
        if let Err(error) = self.notifier.notify(&notification).await {
            tracing::warn!(%error, "subscription notification not delivered, will retry");
            return Ok(());
        }
        let stored = id.clone();
        self.storage
            .run(move |conn| lapses::mark_notified(conn, &stored))
            .await?;
        self.lapse_notified().insert(id);
        Ok(())
    }

    pub fn renewed(&self, id: &AccountId) {
        self.lapse_notified().remove(id);
    }

    fn lapse_notified(&self) -> MutexGuard<'_, HashSet<AccountId>> {
        self.lapses_notified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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
            account_id: &account.id().0,
            provider_name: review.provider_name,
            account_name: account.label.as_deref().or(account.email.as_deref()),
            window,
        };
        for milestone in evaluation.alerts.clone() {
            if !enabled(review.settings, milestone) {
                continue;
            }
            let notification = compose(review.locale, milestone, &subject, observed, review.now);
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

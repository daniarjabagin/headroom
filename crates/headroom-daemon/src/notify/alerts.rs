use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use headroom_core::account::AccountId;
use headroom_core::forecast::{Activity, Signal, forecast};
use headroom_core::history::observed_at;
use headroom_core::quota::{LimitsSnapshot, QuotaWindow};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::Connection;
use tokio::sync::Mutex as AsyncMutex;

use super::Notifier;
use super::evaluator::{AlertState, Evaluation, Milestone, Observation, evaluate, rollback};
use super::held::{Cause, HeldAlert};
use super::quiet::is_quiet_at;
use super::text::{Alert, Locale, Notification, Subject, Urgency, compose, compose_lapse, heading};
use crate::error::StorageError;
use crate::model::window_key;
use crate::quota_history::{WindowSamples, window_samples};
use crate::settings::{DisplaySettings, NotificationSettings};
use crate::storage::Storage;
use crate::storage::accounts::AccountRecord;
use crate::storage::{alerts, held, lapses};

#[path = "alerts_release.rs"]
mod release;

pub use release::Release;

type AlertKey = (AccountId, String);

pub struct Alerts {
    states: Mutex<HashMap<AlertKey, AlertState>>,
    lapses_notified: Mutex<HashSet<AccountId>>,
    held: AsyncMutex<BTreeMap<String, HeldAlert>>,
    notifier: Arc<dyn Notifier>,
    storage: Storage,
}

#[derive(Debug, Clone, Copy)]
struct Gate {
    quiet: bool,
    allow_critical: bool,
}

impl Gate {
    fn of(settings: &NotificationSettings, now: Timestamp, tz: &TimeZone) -> Gate {
        Gate {
            quiet: is_quiet_at(now, tz, settings.quiet_hours),
            allow_critical: settings.quiet_hours.allow_critical,
        }
    }

    fn holds(self, urgency: Urgency) -> bool {
        self.quiet && !(self.allow_critical && urgency == Urgency::Critical)
    }
}

pub struct Review<'a> {
    pub account: &'a AccountRecord,
    pub provider_name: &'a str,
    pub snapshot: &'a LimitsSnapshot,
    pub history: Option<&'a WindowSamples>,
    pub signal: Signal,
    pub settings: NotificationSettings,
    pub display: &'a DisplaySettings,
    pub locale: Locale,
    pub now: Timestamp,
    pub tz: &'a TimeZone,
}

pub struct LapseReview<'a> {
    pub account: &'a AccountRecord,
    pub provider_name: &'a str,
    pub settings: &'a NotificationSettings,
    pub locale: Locale,
    pub now: Timestamp,
    pub tz: &'a TimeZone,
}

impl LapseReview<'_> {
    fn gate(&self) -> Gate {
        Gate::of(self.settings, self.now, self.tz)
    }
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
        let held = held::load_all::<HeldAlert>(conn)?
            .into_iter()
            .map(|alert| (alert.id().to_owned(), alert))
            .collect();
        Ok(Alerts {
            states: Mutex::new(states),
            lapses_notified: Mutex::new(lapses_notified),
            held: AsyncMutex::new(held),
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

    pub async fn review_lapse(&self, lapse: &LapseReview<'_>) -> Result<(), StorageError> {
        let account = lapse.account;
        let id = account.id().clone();
        if !account.is_visible() || self.lapse_notified().contains(&id) {
            return Ok(());
        }
        let name = account.label.as_deref().or(account.email.as_deref());
        let notification = compose_lapse(lapse.locale, &id.0, lapse.provider_name, name);
        let held = HeldAlert {
            heading: headed(lapse.provider_name, name),
            notification,
            cause: Cause::Lapse,
            held_at: lapse.now,
        };
        if !self.dispatch(held, lapse.gate()).await {
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
        let activity = Activity {
            samples: window_samples(review.history, &key.1),
            signal: review.signal,
            observed_at: observed_at(review.snapshot),
        };
        let pace = forecast(window, activity, review.now);
        let observed = Observation::of(window, &pace, review.now);
        let previous = self.state(&key);
        let threshold = threshold_for(&review.settings, review.account.reference.provider.as_str());
        let mut evaluation = evaluate(previous.as_ref(), &observed, threshold);
        self.deliver(review, window, &observed, threshold, &mut evaluation)
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
        threshold: u8,
        evaluation: &mut Evaluation,
    ) {
        let account = review.account;
        let subject = Subject {
            account_id: &account.id().0,
            provider_name: review.provider_name,
            account_name: account.label.as_deref().or(account.email.as_deref()),
            window,
        };
        let gate = Gate::of(&review.settings, review.now, review.tz);
        for milestone in evaluation.alerts.clone() {
            if !enabled(&review.settings, milestone) {
                continue;
            }
            let alert = Alert {
                milestone,
                threshold,
            };
            let held = HeldAlert {
                notification: compose(review.locale, alert, &subject, observed, review.now),
                heading: heading(review.locale, &subject),
                cause: Cause::Window {
                    provider: account.reference.provider.as_str().to_owned(),
                    window: window_key(&window.id),
                    alert,
                    observed: *observed,
                },
                held_at: review.now,
            };
            if !self.dispatch(held, gate).await {
                rollback(&mut evaluation.state, milestone);
            }
        }
    }

    async fn dispatch(&self, held: HeldAlert, gate: Gate) -> bool {
        if gate.holds(held.notification.urgency) {
            return self.hold(held).await;
        }
        self.send(&held.notification).await
    }

    async fn send(&self, notification: &Notification) -> bool {
        match self.notifier.notify(notification).await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(%error, id = %notification.id, "notification not delivered, will retry");
                false
            }
        }
    }

    async fn hold(&self, alert: HeldAlert) -> bool {
        let mut pending = self.held.lock().await;
        let stored = alert.clone();
        let saved = self
            .storage
            .run(move |conn| held::save(conn, stored.id(), stored.held_at, &stored))
            .await;
        match saved {
            Ok(()) => {
                pending.insert(alert.id().to_owned(), alert);
                true
            }
            Err(error) => {
                tracing::warn!(%error, "could not hold a notification during quiet hours");
                false
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

#[must_use]
pub fn threshold_for(settings: &NotificationSettings, provider: &str) -> u8 {
    settings
        .provider_thresholds
        .get(provider)
        .copied()
        .unwrap_or(settings.threshold_percent)
}

fn headed(provider: &str, account_name: Option<&str>) -> String {
    match account_name {
        Some(name) => format!("{provider} · {name}"),
        None => provider.to_owned(),
    }
}

fn enabled(settings: &NotificationSettings, milestone: Milestone) -> bool {
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

#[cfg(test)]
#[path = "alerts_quiet_tests.rs"]
mod quiet_tests;

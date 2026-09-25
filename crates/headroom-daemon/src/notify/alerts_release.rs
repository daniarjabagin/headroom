use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{Alerts, enabled, threshold_for};
use crate::error::StorageError;
use crate::notify::digest::compose_digest;
use crate::notify::evaluator::Milestone;
use crate::notify::held::{Cause, HeldAlert, window_still_holds};
use crate::notify::quiet::is_quiet_at;
use crate::notify::text::Locale;
use crate::settings::NotificationSettings;
use crate::storage::held;

pub struct Release<'a> {
    pub settings: &'a NotificationSettings,
    pub locale: Locale,
    pub now: Timestamp,
    pub tz: &'a TimeZone,
}

impl Alerts {
    pub async fn release(&self, release: &Release<'_>) -> Result<(), StorageError> {
        if is_quiet_at(release.now, release.tz, release.settings.quiet_hours) {
            return Ok(());
        }
        let mut pending = self.held.lock().await;
        if pending.is_empty() {
            return Ok(());
        }
        let current: Vec<HeldAlert> = pending
            .values()
            .filter(|alert| self.still_pending(alert, release))
            .cloned()
            .collect();
        if let Some(notification) = compose_digest(release.locale, current, release.now)
            && !self.send(&notification).await
        {
            return Ok(());
        }
        let ids: Vec<String> = pending.keys().cloned().collect();
        self.storage
            .run(move |conn| held::delete(conn, &ids))
            .await?;
        pending.clear();
        Ok(())
    }

    #[cfg(test)]
    pub(crate) async fn held_ids(&self) -> Vec<String> {
        self.held.lock().await.keys().cloned().collect()
    }

    fn still_pending(&self, alert: &HeldAlert, release: &Release<'_>) -> bool {
        let Cause::Window {
            provider,
            window,
            alert: fired,
            observed,
        } = &alert.cause
        else {
            return self.lapse_notified().contains(&alert.account());
        };
        let settings = release.settings;
        let switched_off = !enabled(settings, fired.milestone)
            || (fired.milestone == Milestone::AlmostOut && threshold_for(settings, provider) == 0);
        if switched_off {
            return false;
        }
        let state = self.state(&(alert.account(), window.clone()));
        window_still_holds(fired.milestone, observed, state.as_ref(), release.now)
    }
}

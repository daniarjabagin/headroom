use headroom_core::account::AccountId;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::evaluator::{AlertState, Milestone, Observation, has_reset};
use super::text::{Alert, Notification};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeldAlert {
    pub notification: Notification,
    pub heading: String,
    pub cause: Cause,
    pub held_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cause {
    Window {
        provider: String,
        window: String,
        alert: Alert,
        observed: Observation,
    },
    Lapse,
}

impl HeldAlert {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.notification.id
    }

    #[must_use]
    pub fn account(&self) -> AccountId {
        AccountId(self.notification.account_id.clone())
    }
}

#[must_use]
pub fn window_still_holds(
    milestone: Milestone,
    observed: &Observation,
    current: Option<&AlertState>,
    now: Timestamp,
) -> bool {
    let Some(current) = current else {
        return false;
    };
    if observed.resets_at.is_some_and(|at| at <= now)
        || has_reset(observed.resets_at, current.resets_at)
    {
        return false;
    }
    milestone == Milestone::Reset || current.fired.contains(&milestone)
}

#[cfg(test)]
#[path = "held_tests.rs"]
mod tests;

use std::sync::Arc;

use headroom_core::account::AccountId;

use crate::core::Core;
use crate::error::CommandError;
use crate::rescan::Rescans;
use crate::update::{CheckOutcome, UpdateChecks};

#[derive(Clone)]
pub struct Service {
    core: Arc<Core>,
    rescans: Rescans,
    update_checks: Option<UpdateChecks>,
}

impl Service {
    #[must_use]
    pub fn new(core: Arc<Core>, rescans: Rescans) -> Service {
        Service {
            core,
            rescans,
            update_checks: None,
        }
    }

    #[must_use]
    pub fn with_update_checks(self, update_checks: UpdateChecks) -> Service {
        Service {
            update_checks: Some(update_checks),
            ..self
        }
    }

    #[must_use]
    pub fn core(&self) -> &Core {
        &self.core
    }

    pub async fn rescan(&self) -> Result<(), CommandError> {
        self.rescans.rescan().await
    }

    pub async fn check_for_updates(&self) -> Result<CheckOutcome, CommandError> {
        match &self.update_checks {
            Some(checks) => checks.check().await,
            None => Err(CommandError::UpdateChecksUnavailable),
        }
    }

    pub fn refresh(&self, account_id: &str) -> Result<(), CommandError> {
        let Some(id) = self.core.begin_recovery(account_id)? else {
            return self.core.refresh(account_id);
        };
        let service = self.clone();
        tokio::spawn(async move { service.recover(&id).await });
        Ok(())
    }

    async fn recover(&self, id: &AccountId) {
        if let Err(error) = self.rescans.rescan().await {
            tracing::debug!(account = %id, %error, "rescan before a retry failed");
        }
        if let Err(error) = self.core.finish_recovery(id) {
            tracing::debug!(account = %id, %error, "retry after a rescan failed");
        }
    }

    pub async fn dismiss_account(&self, account_id: &str) -> Result<(), CommandError> {
        self.core.dismiss_account(account_id).await?;
        self.rescans.rescan().await
    }

    pub async fn restore_accounts(&self, provider: &str) -> Result<(), CommandError> {
        self.core.restore_accounts(provider).await?;
        self.rescans.rescan().await
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

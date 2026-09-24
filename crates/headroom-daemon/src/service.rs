use std::sync::Arc;

use crate::core::Core;
use crate::error::CommandError;
use crate::rescan::Rescans;

#[derive(Clone)]
pub struct Service {
    core: Arc<Core>,
    rescans: Rescans,
}

impl Service {
    #[must_use]
    pub fn new(core: Arc<Core>, rescans: Rescans) -> Service {
        Service { core, rescans }
    }

    #[must_use]
    pub fn core(&self) -> &Core {
        &self.core
    }

    pub async fn rescan(&self) -> Result<(), CommandError> {
        self.rescans.rescan().await
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

use headroom_core::account::{AccountId, CredentialOwner, ProviderId};

use crate::core::Core;
use crate::dismissed::DismissedHome;
use crate::error::CommandError;
use crate::storage::dismissed;

impl Core {
    pub async fn dismiss_account(&self, account_id: &str) -> Result<(), CommandError> {
        let _write = self.settings_write.lock().await;
        let Some(home) = self.dismissable(account_id)? else {
            return Ok(());
        };
        let stored = home.clone();
        self.storage
            .run(move |conn| dismissed::insert(conn, &stored))
            .await?;
        self.model().dismissed.insert(home);
        self.mark_changed();
        Ok(())
    }

    pub async fn restore_accounts(&self, provider: &str) -> Result<(), CommandError> {
        let _write = self.settings_write.lock().await;
        let provider = self.restorable(provider)?;
        let stored = provider.clone();
        self.storage
            .run(move |conn| dismissed::restore(conn, stored.as_ref()))
            .await?;
        self.model().dismissed.restore(provider.as_ref());
        self.mark_changed();
        Ok(())
    }

    fn dismissable(&self, account_id: &str) -> Result<Option<DismissedHome>, CommandError> {
        let id = AccountId(account_id.to_owned());
        let model = self.model();
        let Some(record) = model.stored_account(&id) else {
            return Err(CommandError::UnknownAccount(account_id.to_owned()));
        };
        if model.dismissed.hides(&record.reference) {
            return Ok(None);
        }
        if record.gone {
            return Err(CommandError::UnknownAccount(account_id.to_owned()));
        }
        match record.reference.owner {
            CredentialOwner::Cli => Ok(Some(DismissedHome::of(&record.reference))),
            CredentialOwner::Headroom => Err(CommandError::NotDismissable(account_id.to_owned())),
        }
    }

    fn restorable(&self, provider: &str) -> Result<Option<ProviderId>, CommandError> {
        if provider.is_empty() {
            return Ok(None);
        }
        match self.providers.iter().find(|p| p.id().as_str() == provider) {
            Some(known) => Ok(Some(known.id().clone())),
            None => Err(CommandError::UnknownProvider(provider.to_owned())),
        }
    }
}

#[cfg(test)]
#[path = "dismissal_tests.rs"]
mod tests;

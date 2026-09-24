use headroom_core::account::{AccountId, CredentialOwner};

use crate::core::Core;
use crate::error::CommandError;

impl Core {
    pub async fn dismiss_account(&self, account_id: &str) -> Result<(), CommandError> {
        let _write = self.settings_write.lock().await;
        self.dismissable(account_id)?;
        let mut settings = self.model().settings.clone();
        settings.dismiss(account_id);
        self.store_settings(settings).await
    }

    pub async fn restore_accounts(&self, provider: &str) -> Result<(), CommandError> {
        let _write = self.settings_write.lock().await;
        let provider = self.restorable(provider)?;
        let mut settings = self.model().settings.clone();
        settings.restore(provider);
        self.store_settings(settings).await
    }

    fn dismissable(&self, account_id: &str) -> Result<(), CommandError> {
        let id = AccountId(account_id.to_owned());
        let model = self.model();
        let Some(record) = model.discovered_account(&id) else {
            return Err(CommandError::UnknownAccount(account_id.to_owned()));
        };
        match record.reference.owner {
            CredentialOwner::Cli => Ok(()),
            CredentialOwner::Headroom => Err(CommandError::NotDismissable(account_id.to_owned())),
        }
    }

    fn restorable<'a>(&self, provider: &'a str) -> Result<Option<&'a str>, CommandError> {
        if provider.is_empty() {
            return Ok(None);
        }
        if self.providers.iter().any(|p| p.id().as_str() == provider) {
            Ok(Some(provider))
        } else {
            Err(CommandError::UnknownProvider(provider.to_owned()))
        }
    }
}

#[cfg(test)]
#[path = "dismissal_tests.rs"]
mod tests;

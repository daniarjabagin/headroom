use std::collections::HashSet;

use headroom_core::account::AccountId;

use crate::core::Core;
use crate::error::CommandError;
use crate::scheduler::policy;
use crate::settings::Settings;
use crate::storage::accounts;

pub const MAX_LABEL_CHARS: usize = 64;

impl Core {
    pub fn refresh(&self, account_id: &str) -> Result<(), CommandError> {
        if account_id.is_empty() {
            self.refresh_due();
            return Ok(());
        }
        let id = self.known_account(account_id)?;
        self.trigger(&id);
        Ok(())
    }

    fn refresh_due(&self) {
        let now = self.clock.now();
        let due: Vec<AccountId> = {
            let model = self.model();
            model
                .active_accounts()
                .filter(|a| policy::soft_refresh_due(model.runtime.get(a.id()), now))
                .map(|a| a.id().clone())
                .collect()
        };
        for id in &due {
            self.trigger(id);
        }
    }

    pub fn settings_json(&self) -> Result<String, CommandError> {
        Ok(serde_json::to_string(&self.model().settings)?)
    }

    pub async fn set_settings(&self, json: &str) -> Result<(), CommandError> {
        let _write = self.settings_write.lock().await;
        self.store_settings(Settings::parse(json)?).await
    }

    pub async fn update_settings(&self, patch: &str) -> Result<(), CommandError> {
        let _write = self.settings_write.lock().await;
        let current = self.model().settings.clone();
        self.store_settings(current.patched(patch)?).await
    }

    async fn store_settings(&self, settings: Settings) -> Result<(), CommandError> {
        let stored = settings.clone();
        self.storage
            .run(move |conn| crate::storage::settings::save(conn, &stored))
            .await?;
        self.model().settings = settings;
        self.mark_changed();
        Ok(())
    }

    pub async fn set_account_label(
        &self,
        account_id: &str,
        label: &str,
    ) -> Result<(), CommandError> {
        let id = self.known_account(account_id)?;
        let label = normalize_label(label)?;
        self.storage
            .run(move |conn| accounts::set_label(conn, &id, label.as_deref()))
            .await?;
        Ok(self.reload_accounts().await?)
    }

    pub async fn set_account_hidden(
        &self,
        account_id: &str,
        hidden: bool,
    ) -> Result<(), CommandError> {
        let id = self.known_account(account_id)?;
        self.storage
            .run(move |conn| accounts::set_hidden(conn, &id, hidden))
            .await?;
        Ok(self.reload_accounts().await?)
    }

    pub async fn set_account_order(&self, ids: &[String]) -> Result<(), CommandError> {
        let current: Vec<AccountId> = self
            .model()
            .accounts
            .iter()
            .map(|a| a.id().clone())
            .collect();
        let order = reorder(&current, ids)?;
        self.storage
            .run(move |conn| accounts::set_order(conn, &order))
            .await?;
        Ok(self.reload_accounts().await?)
    }

    fn known_account(&self, account_id: &str) -> Result<AccountId, CommandError> {
        let id = AccountId(account_id.to_owned());
        match self.model().account(&id) {
            Some(_) => Ok(id),
            None => Err(CommandError::UnknownAccount(account_id.to_owned())),
        }
    }
}

fn normalize_label(label: &str) -> Result<Option<String>, CommandError> {
    let trimmed = label.trim();
    if trimmed.chars().count() > MAX_LABEL_CHARS {
        return Err(CommandError::LabelTooLong(MAX_LABEL_CHARS));
    }
    Ok((!trimmed.is_empty()).then(|| trimmed.to_owned()))
}

fn reorder(current: &[AccountId], requested: &[String]) -> Result<Vec<AccountId>, CommandError> {
    let mut seen = HashSet::new();
    let mut order = Vec::with_capacity(current.len());
    for raw in requested {
        let id = AccountId(raw.clone());
        if !current.contains(&id) {
            return Err(CommandError::UnknownAccount(raw.clone()));
        }
        if !seen.insert(id.clone()) {
            return Err(CommandError::DuplicateAccount(raw.clone()));
        }
        order.push(id);
    }
    order.extend(current.iter().filter(|id| !seen.contains(*id)).cloned());
    Ok(order)
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

use std::io::{self, Write};

use anyhow::Result;
use headroom_core::account::{AccountRef, ProviderId};
use headroom_daemon::state::payload::AccountView;
use headroom_providers::registry;

use crate::client::{self, Daemon};
use crate::paths::Globals;
use crate::providers;

pub struct Dismissal {
    daemon: Daemon,
    account: AccountRef,
}

impl Dismissal {
    pub async fn prepare(globals: &Globals, account: &AccountRef) -> Result<Dismissal> {
        let daemon = client::require_daemon(globals).await?;
        Ok(Dismissal {
            daemon,
            account: account.clone(),
        })
    }

    pub async fn apply(self) -> Result<String> {
        let id = &self.account.id.0;
        let (_, state) = client::fetch_state(&self.daemon).await?;
        let shown = state.accounts.into_iter().find(|view| &view.id == id);
        self.daemon.dismiss_account(id).await?;
        Ok(dismissed_message(
            &self.account.provider,
            &shown_label(id, shown),
        ))
    }
}

pub async fn restore(globals: &Globals, provider: Option<&str>) -> Result<()> {
    if let Some(provider) = provider {
        providers::descriptor(provider)?;
    }
    let daemon = client::require_daemon(globals).await?;
    daemon
        .restore_accounts(provider.unwrap_or_default())
        .await?;
    writeln!(io::stdout(), "{}", restored_message(provider))?;
    Ok(())
}

fn shown_label(id: &str, view: Option<AccountView>) -> String {
    view.and_then(|view| view.label.or(view.email))
        .unwrap_or_else(|| id.to_owned())
}

fn provider_name(provider: &ProviderId) -> &str {
    registry::descriptor(provider.as_str()).map_or(provider.as_str(), |d| d.display_name)
}

fn dismissed_message(provider: &ProviderId, label: &str) -> String {
    let name = provider_name(provider);
    format!(
        "Headroom will stop showing {name} account {label}; the {name} CLI stays signed in. \
         Undo with `headroom accounts restore {provider}`."
    )
}

fn restored_message(provider: Option<&str>) -> String {
    match provider {
        Some(provider) => format!("Headroom shows dismissed {provider} accounts again."),
        None => "Headroom shows all dismissed accounts again.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dismissal_names_the_provider_and_keeps_the_cli_signed_in() {
        let grok = ProviderId::parse("grok").unwrap();
        let message = dismissed_message(&grok, "ada@example.com");
        assert!(message.starts_with("Headroom will stop showing Grok account ada@example.com;"));
        assert!(message.contains("the Grok CLI stays signed in"));
        assert!(message.contains("headroom accounts restore grok"));
    }

    #[test]
    fn labels_fall_back_from_label_to_email_to_id() {
        assert_eq!(shown_label("grok:abc", None), "grok:abc");
    }

    #[test]
    fn restore_messages_name_the_scope() {
        assert_eq!(
            restored_message(Some("cline")),
            "Headroom shows dismissed cline accounts again."
        );
        assert_eq!(
            restored_message(None),
            "Headroom shows all dismissed accounts again."
        );
    }
}

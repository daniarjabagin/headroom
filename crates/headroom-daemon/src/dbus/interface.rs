use zbus::fdo;
use zbus::object_server::SignalEmitter;

use crate::core::Core;
use crate::error::CommandError;
use crate::service::Service;

pub struct DaemonInterface {
    service: Service,
}

impl DaemonInterface {
    #[must_use]
    pub fn new(service: Service) -> DaemonInterface {
        DaemonInterface { service }
    }

    fn core(&self) -> &Core {
        self.service.core()
    }
}

#[zbus::interface(name = "io.github.headroom.Daemon1")]
impl DaemonInterface {
    fn get_state(&self) -> fdo::Result<String> {
        self.core()
            .state_json()
            .map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    fn refresh(&self, account_id: &str) -> fdo::Result<()> {
        self.core()
            .refresh(account_id)
            .map_err(|error| to_fdo(&error))
    }

    fn refresh_now(&self) {
        self.core().refresh_now();
    }

    async fn rescan(&self) -> fdo::Result<()> {
        self.service.rescan().await.map_err(|error| to_fdo(&error))
    }

    fn list_providers(&self) -> fdo::Result<String> {
        self.core()
            .providers_json()
            .map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    fn get_settings(&self) -> fdo::Result<String> {
        self.core().settings_json().map_err(|error| to_fdo(&error))
    }

    async fn set_settings(&self, json: &str) -> fdo::Result<()> {
        self.core()
            .set_settings(json)
            .await
            .map_err(|error| to_fdo(&error))
    }

    async fn update_settings(&self, patch: &str) -> fdo::Result<()> {
        self.core()
            .update_settings(patch)
            .await
            .map_err(|error| to_fdo(&error))
    }

    async fn set_account_label(&self, account_id: &str, label: &str) -> fdo::Result<()> {
        self.core()
            .set_account_label(account_id, label)
            .await
            .map_err(|error| to_fdo(&error))
    }

    async fn set_account_order(&self, ids: Vec<String>) -> fdo::Result<()> {
        self.core()
            .set_account_order(&ids)
            .await
            .map_err(|error| to_fdo(&error))
    }

    async fn set_account_hidden(&self, account_id: &str, hidden: bool) -> fdo::Result<()> {
        self.core()
            .set_account_hidden(account_id, hidden)
            .await
            .map_err(|error| to_fdo(&error))
    }

    async fn dismiss_account(&self, account_id: &str) -> fdo::Result<()> {
        self.service
            .dismiss_account(account_id)
            .await
            .map_err(|error| to_fdo(&error))
    }

    async fn restore_accounts(&self, provider: &str) -> fdo::Result<()> {
        self.service
            .restore_accounts(provider)
            .await
            .map_err(|error| to_fdo(&error))
    }

    #[zbus(signal)]
    pub async fn state_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn open_requested(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

fn to_fdo(error: &CommandError) -> fdo::Error {
    if error.is_invalid_argument() {
        fdo::Error::InvalidArgs(error.to_string())
    } else {
        fdo::Error::Failed(error.to_string())
    }
}

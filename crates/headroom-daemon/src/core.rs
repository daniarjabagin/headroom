use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use headroom_core::account::{AccountId, AccountRef, ProviderId};
use headroom_core::provider::Provider;
use headroom_core::usage::PriceBook;
use jiff::tz::TimeZone;
use tokio::sync::{Mutex as AsyncMutex, Notify, mpsc, watch};

use crate::catalog::ProviderCatalog;
use crate::clock::Clock;
use crate::error::StorageError;
use crate::home::{HomeDisplay, UsageHome};
use crate::model::Model;
use crate::notify::Notifier;
use crate::notify::alerts::Alerts;
use crate::notify::text::Locale;
use crate::random::Random;
use crate::state::payload::StatePayload;
use crate::state::{self, AssembleContext};
use crate::storage::Storage;
use crate::storage::accounts;

pub struct CoreParts {
    pub storage: Storage,
    pub providers: Vec<Arc<dyn Provider>>,
    pub catalog: ProviderCatalog,
    pub price_book: Arc<dyn PriceBook>,
    pub clock: Arc<dyn Clock>,
    pub random: Arc<dyn Random>,
    pub tz: TimeZone,
    pub homes: HomeDisplay,
    pub notifier: Arc<dyn Notifier>,
    pub system_locale: Locale,
}

pub struct Core {
    pub(crate) storage: Storage,
    pub(crate) providers: Vec<Arc<dyn Provider>>,
    pub(crate) catalog: ProviderCatalog,
    pub(crate) price_book: Arc<dyn PriceBook>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) random: Arc<dyn Random>,
    pub(crate) tz: TimeZone,
    pub(crate) alerts: Alerts,
    pub(crate) system_locale: Locale,
    homes: HomeDisplay,
    model: Mutex<Model>,
    pub(crate) settings_write: AsyncMutex<()>,
    pub(crate) log_reads: AsyncMutex<()>,
    changes: Notify,
    triggers: Mutex<HashMap<AccountId, mpsc::Sender<()>>>,
    ingest_requests: watch::Sender<u64>,
}

impl Core {
    pub async fn load(parts: CoreParts) -> Result<Core, StorageError> {
        let storage = parts.storage.clone();
        let notifier = parts.notifier.clone();
        let (model, alerts) = parts
            .storage
            .run(move |conn| Ok((Model::load(conn)?, Alerts::load(conn, storage, notifier)?)))
            .await?;
        Ok(Core {
            storage: parts.storage,
            providers: parts.providers,
            catalog: parts.catalog,
            price_book: parts.price_book,
            clock: parts.clock,
            random: parts.random,
            tz: parts.tz,
            alerts,
            system_locale: parts.system_locale,
            homes: parts.homes,
            model: Mutex::new(model),
            settings_write: AsyncMutex::new(()),
            log_reads: AsyncMutex::new(()),
            changes: Notify::new(),
            triggers: Mutex::new(HashMap::new()),
            ingest_requests: watch::Sender::new(0),
        })
    }

    pub fn model(&self) -> MutexGuard<'_, Model> {
        self.model.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn mark_changed(&self) {
        self.changes.notify_one();
    }

    pub async fn changed(&self) {
        self.changes.notified().await;
    }

    #[must_use]
    pub fn state(&self) -> StatePayload {
        let ctx = AssembleContext {
            now: self.clock.now(),
            tz: &self.tz,
            homes: &self.homes,
            catalog: &self.catalog,
        };
        state::assemble(&self.model(), &ctx)
    }

    pub fn state_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.state())
    }

    pub fn providers_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.catalog.payload())
    }

    #[must_use]
    pub fn provider(&self, id: &ProviderId) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find(|p| p.id() == id).cloned()
    }

    #[must_use]
    pub fn active_accounts(&self) -> Vec<AccountRef> {
        self.model()
            .active_accounts()
            .map(|a| a.reference.clone())
            .collect()
    }

    pub async fn reload_accounts(&self) -> Result<(), StorageError> {
        let accounts = self.storage.run(|conn| accounts::load_all(conn)).await?;
        self.model().accounts = accounts;
        self.mark_changed();
        Ok(())
    }

    pub fn set_usage_homes(&self, provider: &ProviderId, homes: Vec<PathBuf>) {
        let mut model = self.model();
        model.usage_homes.retain(|home| &home.provider != provider);
        model
            .usage_homes
            .extend(homes.into_iter().map(|home| UsageHome {
                provider: provider.clone(),
                home,
            }));
        drop(model);
        self.mark_changed();
    }

    pub(crate) fn register_trigger(&self, id: AccountId, trigger: mpsc::Sender<()>) {
        self.triggers().insert(id, trigger);
    }

    pub(crate) fn remove_trigger(&self, id: &AccountId) {
        self.triggers().remove(id);
    }

    pub(crate) fn trigger(&self, id: &AccountId) {
        self.send_trigger(id);
    }

    pub(crate) fn has_trigger(&self, id: &AccountId) -> bool {
        self.triggers().contains_key(id)
    }

    pub(crate) fn send_trigger(&self, id: &AccountId) -> bool {
        let Some(sender) = self.triggers().get(id).cloned() else {
            return false;
        };
        match sender.try_send(()) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(())) => true,
            Err(mpsc::error::TrySendError::Closed(())) => {
                tracing::debug!(account = %id, "refresh worker is gone");
                false
            }
        }
    }

    pub(crate) fn request_ingest(&self) {
        self.ingest_requests
            .send_modify(|count| *count = count.wrapping_add(1));
    }

    pub(crate) fn ingest_requests(&self) -> watch::Receiver<u64> {
        self.ingest_requests.subscribe()
    }

    fn triggers(&self) -> MutexGuard<'_, HashMap<AccountId, mpsc::Sender<()>>> {
        self.triggers.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

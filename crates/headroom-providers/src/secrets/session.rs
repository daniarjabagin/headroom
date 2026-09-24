use tokio::sync::Mutex;
use zbus::Connection;
use zbus::connection::Builder;

use headroom_core::secret::SecretString;

use super::backend::BackendError;
use super::service::{self, Attributes, ServiceError};
use super::{ForeignSecret, SERVICE_TIMEOUT, SecretBus, SecretError};

pub(super) struct SecretService {
    bus: SecretBus,
    conn: Mutex<Option<Connection>>,
}

impl SecretService {
    pub(super) fn new(bus: SecretBus) -> SecretService {
        SecretService {
            bus,
            conn: Mutex::new(None),
        }
    }

    pub(super) async fn store(
        &self,
        attributes: &Attributes<'_>,
        label: &str,
        secret: &SecretString,
    ) -> Result<(), BackendError> {
        self.with_service(async |conn| service::store(conn, attributes, label, secret).await)
            .await
    }

    pub(super) async fn read(
        &self,
        attributes: &Attributes<'_>,
    ) -> Result<Option<SecretString>, BackendError> {
        self.with_service(async |conn| service::read(conn, attributes).await)
            .await
    }

    pub(super) async fn delete(&self, attributes: &Attributes<'_>) -> Result<(), BackendError> {
        self.with_service(async |conn| service::delete(conn, attributes).await)
            .await
    }

    async fn with_service<T>(
        &self,
        operation: impl AsyncFnOnce(&Connection) -> Result<T, ServiceError>,
    ) -> Result<T, BackendError> {
        let Some(conn) = self.connection().await else {
            return Err(BackendError::Unavailable);
        };
        if let Ok(result) = tokio::time::timeout(SERVICE_TIMEOUT, operation(&conn)).await {
            result.map_err(backend_error)
        } else {
            tracing::debug!("the Secret Service did not answer in time");
            Err(BackendError::Unavailable)
        }
    }

    async fn connection(&self) -> Option<Connection> {
        let mut cached = self.conn.lock().await;
        if let Some(conn) = cached.as_ref() {
            return Some(conn.clone());
        }
        let conn = connect(&self.bus).await?;
        *cached = Some(conn.clone());
        Some(conn)
    }
}

pub(super) async fn read_foreign(bus: &SecretBus, attributes: &[(&str, &str)]) -> ForeignSecret {
    let Some(conn) = connect(bus).await else {
        return ForeignSecret::Absent;
    };
    let attributes: Attributes = attributes.iter().copied().collect();
    match tokio::time::timeout(SERVICE_TIMEOUT, service::lookup(&conn, &attributes)).await {
        Ok(Ok(found)) => found,
        Ok(Err(error)) => {
            tracing::debug!(?error, "Secret Service lookup failed");
            ForeignSecret::Absent
        }
        Err(_) => {
            tracing::debug!("the Secret Service did not answer in time");
            ForeignSecret::Absent
        }
    }
}

async fn connect(bus: &SecretBus) -> Option<Connection> {
    let builder = match bus {
        SecretBus::Session => Builder::session(),
        SecretBus::Address(address) => Builder::address(address.as_str()),
        SecretBus::Disabled => return None,
    };
    let connected = tokio::time::timeout(SERVICE_TIMEOUT, async { builder?.build().await }).await;
    match connected {
        Ok(Ok(conn)) => Some(conn),
        Ok(Err(error)) => {
            tracing::debug!(%error, "no session bus for the Secret Service");
            None
        }
        Err(_) => None,
    }
}

fn backend_error(error: ServiceError) -> BackendError {
    match error {
        ServiceError::Unavailable => BackendError::Unavailable,
        ServiceError::Locked => BackendError::Locked,
        ServiceError::Failed(error) => {
            BackendError::Failed(SecretError::Service(error.to_string()))
        }
    }
}

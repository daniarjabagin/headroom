use std::sync::Arc;

use zbus::Connection;

use crate::config::BusTarget;
use crate::dbus::{self, signals::BusSignals};
use crate::error::DaemonError;
use crate::events::{EventSink, EventSinks};
use crate::notify::Notifier;
use crate::notify::desktop::DesktopNotifier;
use crate::service::Service;

pub struct BusTransport {
    conn: Connection,
    notifier: Arc<DesktopNotifier>,
}

impl BusTransport {
    pub async fn connect(target: &BusTarget) -> Result<BusTransport, DaemonError> {
        let conn = dbus::connect(target).await?;
        let notifier = Arc::new(DesktopNotifier::new(&conn).await?);
        Ok(BusTransport { conn, notifier })
    }

    pub fn notifier(&self) -> Arc<dyn Notifier> {
        self.notifier.clone()
    }

    pub async fn serve(&self, service: Service, sinks: &mut EventSinks) -> Result<(), DaemonError> {
        dbus::serve(&self.conn, service).await?;
        sinks.push(Arc::new(BusSignals::new(self.conn.clone())));
        tracing::info!(name = dbus::BUS_NAME, "serving on the session bus");
        Ok(())
    }

    pub async fn forward_actions(self, sink: Arc<dyn EventSink>) {
        if let Err(error) = self.notifier.forward_actions(sink).await {
            tracing::warn!(%error, "stopped listening for notification actions");
        }
    }
}

use async_trait::async_trait;
use zbus::Connection;
use zbus::object_server::SignalEmitter;

use super::OBJECT_PATH;
use super::interface::DaemonInterface;

#[async_trait]
pub trait SignalSink: Send + Sync {
    async fn state_changed(&self, state: &str) -> zbus::Result<()>;
    async fn open_requested(&self) -> zbus::Result<()>;
}

pub struct BusSignals {
    conn: Connection,
}

impl BusSignals {
    #[must_use]
    pub fn new(conn: Connection) -> BusSignals {
        BusSignals { conn }
    }

    fn emitter(&self) -> zbus::Result<SignalEmitter<'static>> {
        SignalEmitter::new(&self.conn, OBJECT_PATH)
    }
}

#[async_trait]
impl SignalSink for BusSignals {
    async fn state_changed(&self, state: &str) -> zbus::Result<()> {
        DaemonInterface::state_changed(&self.emitter()?, state).await
    }

    async fn open_requested(&self) -> zbus::Result<()> {
        DaemonInterface::open_requested(&self.emitter()?).await
    }
}

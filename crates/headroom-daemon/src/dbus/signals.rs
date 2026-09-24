use async_trait::async_trait;
use zbus::Connection;
use zbus::object_server::SignalEmitter;

use super::OBJECT_PATH;
use super::interface::DaemonInterface;
use crate::events::{EventError, EventSink};

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
impl EventSink for BusSignals {
    async fn state_changed(&self, state: &str) -> Result<(), EventError> {
        Ok(DaemonInterface::state_changed(&self.emitter()?, state).await?)
    }

    async fn open_requested(&self) -> Result<(), EventError> {
        Ok(DaemonInterface::open_requested(&self.emitter()?).await?)
    }
}

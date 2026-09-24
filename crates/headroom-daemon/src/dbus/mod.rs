pub mod interface;
pub mod signals;

use zbus::Connection;
use zbus::connection::Builder;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use crate::config::BusTarget;
use crate::error::DaemonError;
use crate::service::Service;
use interface::DaemonInterface;

pub const BUS_NAME: &str = "io.github.headroom.Daemon";
pub const OBJECT_PATH: &str = "/io/github/headroom/Daemon";

pub async fn connect(target: &BusTarget) -> Result<Connection, DaemonError> {
    let builder = match target {
        BusTarget::Session => Builder::session()?,
        BusTarget::Address(address) => Builder::address(address.as_str())?,
    };
    Ok(builder.build().await?)
}

pub async fn serve(conn: &Connection, service: Service) -> Result<(), DaemonError> {
    conn.object_server()
        .at(OBJECT_PATH, DaemonInterface::new(service))
        .await?;
    let reply = conn
        .request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await
        .map_err(|error| match error {
            zbus::Error::NameTaken => DaemonError::AlreadyRunning,
            other => DaemonError::Bus(other),
        })?;
    match reply {
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(()),
        RequestNameReply::InQueue | RequestNameReply::Exists => Err(DaemonError::AlreadyRunning),
    }
}

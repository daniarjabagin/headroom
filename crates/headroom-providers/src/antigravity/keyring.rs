use std::collections::HashMap;
use std::time::Duration;

use zbus::Connection;
use zbus::connection::Builder;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::secrets::SecretBus;

const DESTINATION: &str = "org.freedesktop.secrets";
const SERVICE_PATH: &str = "/org/freedesktop/secrets";
const SERVICE: &str = "org.freedesktop.Secret.Service";
const ITEM: &str = "org.freedesktop.Secret.Item";
const SESSION: &str = "org.freedesktop.Secret.Session";
const TIMEOUT: Duration = Duration::from_secs(10);
const ATTRIBUTES: [(&str, &str); 2] = [("service", "gemini"), ("username", "antigravity")];

type Paths = Vec<OwnedObjectPath>;
type Secret = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum KeyringItem {
    Found(String),
    Locked,
    Absent,
}

pub(super) async fn read(bus: &SecretBus) -> KeyringItem {
    match tokio::time::timeout(TIMEOUT, lookup(bus)).await {
        Ok(Ok(item)) => item,
        Ok(Err(error)) => {
            tracing::debug!(%error, "antigravity keyring lookup failed");
            KeyringItem::Absent
        }
        Err(_) => {
            tracing::debug!("the Secret Service did not answer in time");
            KeyringItem::Absent
        }
    }
}

async fn lookup(bus: &SecretBus) -> zbus::Result<KeyringItem> {
    let builder = match bus {
        SecretBus::Session => Builder::session()?,
        SecretBus::Address(address) => Builder::address(address.as_str())?,
        SecretBus::Disabled => return Ok(KeyringItem::Absent),
    };
    let conn = builder.build().await?;
    let attributes: HashMap<&str, &str> = HashMap::from(ATTRIBUTES);
    let (unlocked, locked): (Paths, Paths) = conn
        .call_method(
            Some(DESTINATION),
            SERVICE_PATH,
            Some(SERVICE),
            "SearchItems",
            &(attributes,),
        )
        .await?
        .body()
        .deserialize()?;
    match (unlocked.first(), locked.is_empty()) {
        (Some(item), _) => secret(&conn, item).await.map(KeyringItem::Found),
        (None, false) => Ok(KeyringItem::Locked),
        (None, true) => Ok(KeyringItem::Absent),
    }
}

async fn secret(conn: &Connection, item: &OwnedObjectPath) -> zbus::Result<String> {
    let (_, session): (OwnedValue, OwnedObjectPath) = conn
        .call_method(
            Some(DESTINATION),
            SERVICE_PATH,
            Some(SERVICE),
            "OpenSession",
            &("plain", Value::new("")),
        )
        .await?
        .body()
        .deserialize()?;
    let fetched = conn
        .call_method(
            Some(DESTINATION),
            item.as_str(),
            Some(ITEM),
            "GetSecret",
            &(&session,),
        )
        .await
        .and_then(|reply| reply.body().deserialize::<Secret>());
    if let Err(error) = conn
        .call_method(
            Some(DESTINATION),
            session.as_str(),
            Some(SESSION),
            "Close",
            &(),
        )
        .await
    {
        tracing::debug!(%error, "could not close the Secret Service session");
    }
    let (_, _, value, _) = fetched?;
    String::from_utf8(value)
        .map_err(|_| zbus::Error::Failure("the Antigravity keyring item is not UTF-8".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_disabled_bus_has_no_item() {
        assert_eq!(read(&SecretBus::Disabled).await, KeyringItem::Absent);
    }

    #[tokio::test]
    async fn an_unreachable_bus_has_no_item() {
        let root = tempfile::tempdir().unwrap();
        let address = format!("unix:path={}", root.path().join("no-bus").display());
        assert_eq!(
            read(&SecretBus::Address(address)).await,
            KeyringItem::Absent
        );
    }
}

use std::collections::HashMap;

use headroom_core::secret::SecretString;
use serde::de::DeserializeOwned;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, StructureBuilder, Type, Value};
use zbus::{Connection, Message};

use super::file::decode;

const DESTINATION: &str = "org.freedesktop.secrets";
const SERVICE_PATH: &str = "/org/freedesktop/secrets";
const SERVICE: &str = "org.freedesktop.Secret.Service";
const COLLECTION: &str = "org.freedesktop.Secret.Collection";
const ITEM: &str = "org.freedesktop.Secret.Item";
const SESSION: &str = "org.freedesktop.Secret.Session";
const DEFAULT_ALIAS: &str = "default";
const NO_OBJECT: &str = "/";
const PLAIN: &str = "plain";
const CONTENT_TYPE: &str = "text/plain; charset=utf8";
const LABEL_PROPERTY: &str = "org.freedesktop.Secret.Item.Label";
const ATTRIBUTES_PROPERTY: &str = "org.freedesktop.Secret.Item.Attributes";

pub(super) type Attributes<'a> = HashMap<&'a str, &'a str>;
type Paths = Vec<OwnedObjectPath>;

#[derive(Debug)]
pub(super) enum ServiceError {
    Unavailable,
    Locked,
    Failed(zbus::Error),
}

impl From<zbus::Error> for ServiceError {
    fn from(error: zbus::Error) -> ServiceError {
        ServiceError::Failed(error)
    }
}

type Secret = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

struct Target<'a> {
    path: &'a str,
    interface: &'static str,
    method: &'static str,
}

impl<'a> Target<'a> {
    fn service(method: &'static str) -> Target<'static> {
        Target {
            path: SERVICE_PATH,
            interface: SERVICE,
            method,
        }
    }

    fn at(path: &'a OwnedObjectPath, interface: &'static str, method: &'static str) -> Target<'a> {
        Target {
            path: path.as_str(),
            interface,
            method,
        }
    }
}

async fn call<R: DeserializeOwned + Type>(
    conn: &Connection,
    target: Target<'_>,
    args: Vec<Value<'_>>,
) -> zbus::Result<R> {
    send(conn, target, args).await?.body().deserialize()
}

async fn send(
    conn: &Connection,
    target: Target<'_>,
    args: Vec<Value<'_>>,
) -> zbus::Result<Message> {
    let Target {
        path,
        interface,
        method,
    } = target;
    if args.is_empty() {
        return conn
            .call_method(Some(DESTINATION), path, Some(interface), method, &())
            .await;
    }
    let body = args
        .into_iter()
        .fold(StructureBuilder::new(), StructureBuilder::append_field)
        .build()?;
    conn.call_method(Some(DESTINATION), path, Some(interface), method, &body)
        .await
}

struct OpenSession<'c> {
    conn: &'c Connection,
    path: OwnedObjectPath,
}

impl<'c> OpenSession<'c> {
    async fn open(conn: &'c Connection) -> Result<OpenSession<'c>, ServiceError> {
        let args = vec![Value::from(PLAIN), Value::new(Value::from(""))];
        let opened: zbus::Result<(OwnedValue, OwnedObjectPath)> =
            call(conn, Target::service("OpenSession"), args).await;
        let (_, path) = opened.map_err(|error| unavailable(&error))?;
        Ok(OpenSession { conn, path })
    }

    fn secret(&self, value: &SecretString) -> zbus::Result<Value<'static>> {
        let secret = StructureBuilder::new()
            .add_field(self.path.clone())
            .add_field(Vec::<u8>::new())
            .add_field(value.expose().as_bytes().to_vec())
            .add_field(CONTENT_TYPE)
            .build()?;
        Ok(Value::from(secret))
    }

    async fn close(self) {
        let target = Target::at(&self.path, SESSION, "Close");
        if let Err(error) = send(self.conn, target, Vec::new()).await {
            tracing::debug!(%error, "could not close the Secret Service session");
        }
    }
}

pub(super) async fn store(
    conn: &Connection,
    attributes: &Attributes<'_>,
    label: &str,
    value: &SecretString,
) -> Result<(), ServiceError> {
    let session = OpenSession::open(conn).await?;
    let stored = create_item(&session, attributes, label, value).await;
    session.close().await;
    stored
}

async fn create_item(
    session: &OpenSession<'_>,
    attributes: &Attributes<'_>,
    label: &str,
    value: &SecretString,
) -> Result<(), ServiceError> {
    let conn = session.conn;
    let alias = vec![Value::from(DEFAULT_ALIAS)];
    let collection: OwnedObjectPath = call(conn, Target::service("ReadAlias"), alias).await?;
    if collection.as_str() == NO_OBJECT {
        return Err(ServiceError::Unavailable);
    }
    unlock(conn, &collection).await?;
    let properties = HashMap::from([
        (LABEL_PROPERTY, Value::from(label)),
        (ATTRIBUTES_PROPERTY, Value::from(attributes.clone())),
    ]);
    let args = vec![
        Value::from(properties),
        session.secret(value)?,
        Value::from(true),
    ];
    let target = Target::at(&collection, COLLECTION, "CreateItem");
    let (_, prompt): (OwnedObjectPath, OwnedObjectPath) = call(conn, target, args).await?;
    no_prompt(&prompt)
}

pub(super) async fn read(
    conn: &Connection,
    attributes: &Attributes<'_>,
) -> Result<Option<SecretString>, ServiceError> {
    let session = OpenSession::open(conn).await?;
    let found = read_item(&session, attributes).await;
    session.close().await;
    found
}

async fn read_item(
    session: &OpenSession<'_>,
    attributes: &Attributes<'_>,
) -> Result<Option<SecretString>, ServiceError> {
    let conn = session.conn;
    let (unlocked, locked) = search(conn, attributes).await?;
    let path = match (unlocked.into_iter().next(), locked.into_iter().next()) {
        (Some(path), _) => path,
        (None, Some(path)) => {
            unlock(conn, &path).await?;
            path
        }
        (None, None) => return Ok(None),
    };
    let args = vec![Value::from(session.path.clone())];
    let secret: Secret = call(conn, Target::at(&path, ITEM, "GetSecret"), args).await?;
    let (_, _, value, _) = secret;
    decode(value).map(Some).map_err(|_| {
        ServiceError::Failed(zbus::Error::Failure("stored secret is not UTF-8".into()))
    })
}

pub(super) async fn delete(
    conn: &Connection,
    attributes: &Attributes<'_>,
) -> Result<(), ServiceError> {
    let (unlocked, locked) = search(conn, attributes)
        .await
        .map_err(|error| unavailable(&error))?;
    for path in unlocked.into_iter().chain(locked) {
        let target = Target::at(&path, ITEM, "Delete");
        let prompt: OwnedObjectPath = call(conn, target, Vec::new()).await?;
        no_prompt(&prompt)?;
    }
    Ok(())
}

async fn search(conn: &Connection, attributes: &Attributes<'_>) -> zbus::Result<(Paths, Paths)> {
    let args = vec![Value::from(attributes.clone())];
    call(conn, Target::service("SearchItems"), args).await
}

async fn unlock(conn: &Connection, path: &OwnedObjectPath) -> Result<(), ServiceError> {
    let args = vec![Value::from(vec![path.clone()])];
    let (unlocked, prompt): (Paths, OwnedObjectPath) =
        call(conn, Target::service("Unlock"), args).await?;
    if unlocked.contains(path) {
        Ok(())
    } else {
        no_prompt(&prompt).and(Err(ServiceError::Locked))
    }
}

fn no_prompt(prompt: &OwnedObjectPath) -> Result<(), ServiceError> {
    if prompt.as_str() == NO_OBJECT {
        Ok(())
    } else {
        Err(ServiceError::Locked)
    }
}

fn unavailable(error: &zbus::Error) -> ServiceError {
    tracing::debug!(%error, "no usable Secret Service");
    ServiceError::Unavailable
}

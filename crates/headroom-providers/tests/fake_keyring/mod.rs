use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use zbus::object_server::ObjectServer;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Type, Value};
use zbus::{fdo, interface};

pub const ROOT: &str = "/org/freedesktop/secrets";
pub const COLLECTION: &str = "/org/freedesktop/secrets/collection/login";
pub const SESSION: &str = "/org/freedesktop/secrets/session/s1";
const PROMPT: &str = "/org/freedesktop/secrets/prompt/p1";

const BUS_CONFIG: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir=DIR</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#;

pub struct PrivateBus {
    child: Child,
    pub address: String,
    _dir: TempDir,
}

impl PrivateBus {
    pub fn start() -> Option<PrivateBus> {
        let dir = tempfile::tempdir().ok()?;
        let config = dir.path().join("bus.conf");
        let text = BUS_CONFIG.replace("DIR", dir.path().to_str()?);
        std::fs::write(&config, text).ok()?;
        let mut child = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config.display()))
            .args(["--nofork", "--print-address"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut line = String::new();
        BufReader::new(child.stdout.take()?)
            .read_line(&mut line)
            .ok()?;
        let address = line.trim().to_owned();
        (!address.is_empty()).then_some(PrivateBus {
            child,
            address,
            _dir: dir,
        })
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
struct Secret(OwnedObjectPath, Vec<u8>, Vec<u8>, String);

#[derive(Debug, Clone, PartialEq)]
pub struct StoredItem {
    pub label: String,
    pub attributes: HashMap<String, String>,
    pub value: Vec<u8>,
    pub content_type: String,
}

#[derive(Default)]
pub struct Keyring {
    pub items: Vec<Option<StoredItem>>,
    pub locked: bool,
    pub no_default: bool,
    pub closed_sessions: usize,
}

pub type Shared = Arc<Mutex<Keyring>>;

pub struct FakeService(pub Shared);
pub struct FakeCollection(pub Shared);
struct FakeItem(Shared, usize);
pub struct FakeSession(pub Shared);

fn path(text: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(text.to_owned()).unwrap()
}

fn item_path(index: usize) -> OwnedObjectPath {
    path(&format!("{COLLECTION}/i{index}"))
}

fn matches(item: &StoredItem, query: &HashMap<String, String>) -> bool {
    query.iter().all(|(k, v)| item.attributes.get(k) == Some(v))
}

#[allow(
    clippy::unused_self,
    clippy::needless_pass_by_value,
    reason = "the Secret Service API fixes these method signatures"
)]
#[interface(name = "org.freedesktop.Secret.Service")]
impl FakeService {
    fn open_session(
        &self,
        algorithm: &str,
        input: Value<'_>,
    ) -> fdo::Result<(OwnedValue, OwnedObjectPath)> {
        if algorithm != "plain" || String::try_from(input).is_err() {
            return Err(fdo::Error::NotSupported(algorithm.into()));
        }
        Ok((OwnedValue::from(0_u32), path(SESSION)))
    }

    fn search_items(
        &self,
        attributes: HashMap<String, String>,
    ) -> (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) {
        let keyring = self.0.lock().unwrap();
        let found: Vec<_> = keyring
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.as_ref().is_some_and(|i| matches(i, &attributes)))
            .map(|(index, _)| item_path(index))
            .collect();
        if keyring.locked {
            (Vec::new(), found)
        } else {
            (found, Vec::new())
        }
    }

    fn unlock(&self, objects: Vec<OwnedObjectPath>) -> (Vec<OwnedObjectPath>, OwnedObjectPath) {
        if self.0.lock().unwrap().locked {
            (Vec::new(), path(PROMPT))
        } else {
            (objects, path("/"))
        }
    }

    fn read_alias(&self, name: &str) -> OwnedObjectPath {
        let keyring = self.0.lock().unwrap();
        if name == "default" && !keyring.no_default {
            path(COLLECTION)
        } else {
            path("/")
        }
    }
}

#[interface(name = "org.freedesktop.Secret.Collection")]
impl FakeCollection {
    async fn create_item(
        &self,
        properties: HashMap<String, OwnedValue>,
        secret: Secret,
        replace: bool,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<(OwnedObjectPath, OwnedObjectPath)> {
        let item = stored_item(&properties, secret)?;
        let index = {
            let mut keyring = self.0.lock().unwrap();
            if replace {
                for slot in &mut keyring.items {
                    if slot
                        .as_ref()
                        .is_some_and(|old| old.attributes == item.attributes)
                    {
                        *slot = None;
                    }
                }
            }
            keyring.items.push(Some(item));
            keyring.items.len() - 1
        };
        server
            .at(item_path(index), FakeItem(self.0.clone(), index))
            .await?;
        Ok((item_path(index), path("/")))
    }
}

fn stored_item(
    properties: &HashMap<String, OwnedValue>,
    secret: Secret,
) -> fdo::Result<StoredItem> {
    let invalid = |e: zbus::zvariant::Error| fdo::Error::InvalidArgs(e.to_string());
    let value = |key: &str| {
        properties
            .get(key)
            .ok_or_else(|| fdo::Error::InvalidArgs(key.into()))
            .and_then(|v| v.try_clone().map_err(invalid))
    };
    let label = String::try_from(value("org.freedesktop.Secret.Item.Label")?).map_err(invalid)?;
    let attributes =
        HashMap::<String, String>::try_from(value("org.freedesktop.Secret.Item.Attributes")?)
            .map_err(invalid)?;
    Ok(StoredItem {
        label,
        attributes,
        value: secret.2,
        content_type: secret.3,
    })
}

#[interface(name = "org.freedesktop.Secret.Item")]
impl FakeItem {
    fn get_secret(&self, session: OwnedObjectPath) -> fdo::Result<Secret> {
        let keyring = self.0.lock().unwrap();
        let item = keyring.items[self.1]
            .clone()
            .ok_or(fdo::Error::UnknownObject("gone".into()))?;
        Ok(Secret(session, Vec::new(), item.value, item.content_type))
    }

    fn delete(&self) -> OwnedObjectPath {
        let mut keyring = self.0.lock().unwrap();
        if keyring.locked {
            return path(PROMPT);
        }
        keyring.items[self.1] = None;
        path("/")
    }
}

#[interface(name = "org.freedesktop.Secret.Session")]
impl FakeSession {
    fn close(&self) {
        self.0.lock().unwrap().closed_sessions += 1;
    }
}

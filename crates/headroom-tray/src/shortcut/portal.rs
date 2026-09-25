use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use futures_util::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use zbus::Connection;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use super::Support;
use super::accelerator::Accelerator;

const DESTINATION: &str = "org.freedesktop.portal.Desktop";
const DESKTOP_PATH: &str = "/org/freedesktop/portal/desktop";
const GLOBAL_SHORTCUTS: &str = "org.freedesktop.portal.GlobalShortcuts";
const REQUEST: &str = "org.freedesktop.portal.Request";
const SESSION: &str = "org.freedesktop.portal.Session";
const REGISTRY: &str = "org.freedesktop.host.portal.Registry";
const SHORTCUT_ID: &str = "open";
const DESCRIPTION: &str = "Open or close the Headroom popup";

type Options = HashMap<String, OwnedValue>;
type Shortcut<'a> = (&'a str, HashMap<&'a str, Value<'a>>);

#[derive(Debug, thiserror::Error)]
pub enum PortalError {
    #[error("the desktop portal call failed: {0}")]
    Bus(#[from] zbus::Error),
    #[error("the desktop portal sent no response")]
    NoResponse,
    #[error("the desktop portal declined the request ({0})")]
    Declined(u32),
    #[error("the desktop portal returned no session")]
    NoSession,
}

#[must_use]
pub fn request_path(unique_name: &str, token: &str) -> String {
    let sender = unique_name.trim_start_matches(':').replace('.', "_");
    format!("{DESKTOP_PATH}/request/{sender}/{token}")
}

#[must_use]
pub fn session_handle(results: &Options) -> Option<OwnedObjectPath> {
    let value = results.get("session_handle")?;
    let text = String::try_from(value.clone()).ok().or_else(|| {
        OwnedObjectPath::try_from(value.clone())
            .ok()
            .map(|path| path.to_string())
    })?;
    OwnedObjectPath::try_from(text).ok()
}

struct Portal {
    connection: Connection,
    next_token: u32,
    session: Option<OwnedObjectPath>,
}

impl Portal {
    fn token(&mut self) -> String {
        self.next_token += 1;
        format!("headroom{}", self.next_token)
    }

    async fn request<B>(
        &mut self,
        method: &str,
        body: &B,
        token: &str,
    ) -> Result<Options, PortalError>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        let unique = self
            .connection
            .unique_name()
            .map(ToString::to_string)
            .unwrap_or_default();
        let request = zbus::Proxy::new(
            &self.connection,
            DESTINATION,
            request_path(&unique, token),
            REQUEST,
        )
        .await?;
        let mut responses = request.receive_signal("Response").await?;
        self.connection
            .call_method(
                Some(DESTINATION),
                DESKTOP_PATH,
                Some(GLOBAL_SHORTCUTS),
                method,
                body,
            )
            .await?;
        let message = responses.next().await.ok_or(PortalError::NoResponse)?;
        let (code, results): (u32, Options) = message.body().deserialize()?;
        if code == 0 {
            Ok(results)
        } else {
            Err(PortalError::Declined(code))
        }
    }

    async fn create_session(&mut self) -> Result<OwnedObjectPath, PortalError> {
        let (token, session_token) = (self.token(), self.token());
        let options = HashMap::from([
            ("handle_token", Value::from(token.as_str())),
            ("session_handle_token", Value::from(session_token.as_str())),
        ]);
        let results = self.request("CreateSession", &(options,), &token).await?;
        session_handle(&results).ok_or(PortalError::NoSession)
    }

    async fn bind(
        &mut self,
        session: &OwnedObjectPath,
        accelerator: &Accelerator,
    ) -> Result<(), PortalError> {
        let token = self.token();
        let trigger = accelerator.portal_trigger();
        let shortcut: Shortcut = (
            SHORTCUT_ID,
            HashMap::from([
                ("description", Value::from(DESCRIPTION)),
                ("preferred_trigger", Value::from(trigger.as_str())),
            ]),
        );
        let options = HashMap::from([("handle_token", Value::from(token.as_str()))]);
        let path = ObjectPath::from(session);
        self.request(
            "BindShortcuts",
            &(path, vec![shortcut], "", options),
            &token,
        )
        .await?;
        Ok(())
    }

    async fn close(&mut self) {
        let Some(session) = self.session.take() else {
            return;
        };
        let closed = self
            .connection
            .call_method(
                Some(DESTINATION),
                session.as_str(),
                Some(SESSION),
                "Close",
                &(),
            )
            .await;
        if let Err(error) = closed {
            tracing::debug!(%error, "could not close the global shortcut session");
        }
    }

    async fn rebind(&mut self, accelerator: Option<Accelerator>) -> Result<(), PortalError> {
        self.close().await;
        let Some(accelerator) = accelerator else {
            return Ok(());
        };
        let session = self.create_session().await?;
        self.session = Some(session.clone());
        self.bind(&session, &accelerator).await
    }
}

fn set_support(support: &Mutex<Support>, value: Support) {
    *support.lock().unwrap_or_else(PoisonError::into_inner) = value;
}

async fn register(connection: &Connection) {
    let registered = connection
        .call_method(
            Some(DESTINATION),
            DESKTOP_PATH,
            Some(REGISTRY),
            "Register",
            &(crate::app::APP_ID, HashMap::<&str, Value>::new()),
        )
        .await;
    if let Err(error) = registered {
        tracing::debug!(%error, "the desktop portal has no host app registry");
    }
}

async fn shortcuts_proxy(connection: &Connection) -> zbus::Result<zbus::Proxy<'static>> {
    let proxy: zbus::Proxy = zbus::proxy::Builder::new(connection)
        .destination(DESTINATION)?
        .path(DESKTOP_PATH)?
        .interface(GLOBAL_SHORTCUTS)?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    proxy.get_property::<u32>("version").await?;
    Ok(proxy)
}

fn is_our_press(portal: &Portal, message: &zbus::Message) -> bool {
    let Ok((session, id, _, _)) = message
        .body()
        .deserialize::<(OwnedObjectPath, String, u64, Options)>()
    else {
        return false;
    };
    id == SHORTCUT_ID && portal.session.as_ref() == Some(&session)
}

async fn run(
    presses: async_channel::Sender<()>,
    support: Arc<Mutex<Support>>,
    mut commands: UnboundedReceiver<Option<Accelerator>>,
) -> Result<(), PortalError> {
    let connection = Connection::session().await?;
    register(&connection).await;
    let proxy = match shortcuts_proxy(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::info!(%error, "global shortcuts are unavailable: no GlobalShortcuts portal");
            set_support(&support, Support::Unsupported);
            return Ok(());
        }
    };
    set_support(&support, Support::Supported);
    let mut activations = proxy.receive_signal("Activated").await?;
    let mut portal = Portal {
        connection,
        next_token: 0,
        session: None,
    };
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(accelerator) = command else {
                    portal.close().await;
                    return Ok(());
                };
                if let Err(error) = portal.rebind(accelerator).await {
                    tracing::warn!(%error, "could not register the global shortcut");
                }
            }
            Some(message) = activations.next() => {
                if is_our_press(&portal, &message) && presses.try_send(()).is_err() {
                    return Ok(());
                }
            }
        }
    }
}

pub struct PortalShortcut {
    commands: UnboundedSender<Option<Accelerator>>,
}

impl PortalShortcut {
    pub fn start(
        presses: async_channel::Sender<()>,
        support: Arc<Mutex<Support>>,
    ) -> std::io::Result<Self> {
        let (commands, receiver) = unbounded_channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let failed = Arc::clone(&support);
        thread::Builder::new()
            .name("headroom-shortcut".into())
            .spawn(move || {
                if let Err(error) = runtime.block_on(run(presses, support, receiver)) {
                    tracing::warn!(%error, "the global shortcut portal stopped");
                    set_support(&failed, Support::Unsupported);
                }
            })?;
        Ok(Self { commands })
    }

    pub fn bind(&self, accelerator: Option<Accelerator>) {
        if self.commands.send(accelerator).is_err() {
            tracing::debug!("the global shortcut portal is gone");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_paths_follow_the_portal_convention() {
        assert_eq!(
            request_path(":1.42", "headroom1"),
            "/org/freedesktop/portal/desktop/request/1_42/headroom1"
        );
    }

    #[test]
    fn session_handles_may_be_strings_or_paths() {
        let path = "/org/freedesktop/portal/desktop/session/1_42/headroom2";
        let as_string = HashMap::from([(
            "session_handle".to_owned(),
            OwnedValue::try_from(Value::from(path)).unwrap(),
        )]);
        assert_eq!(session_handle(&as_string).unwrap().as_str(), path);
        let as_path = HashMap::from([(
            "session_handle".to_owned(),
            OwnedValue::try_from(Value::from(ObjectPath::try_from(path).unwrap())).unwrap(),
        )]);
        assert_eq!(session_handle(&as_path).unwrap().as_str(), path);
        assert!(session_handle(&HashMap::new()).is_none());
    }
}

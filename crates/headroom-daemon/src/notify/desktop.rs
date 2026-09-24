use std::collections::{HashMap, HashSet};
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use zbus::Connection;
use zbus::export::futures_core::Stream;
use zbus::proxy::CacheProperties;
use zbus::zvariant::Value;

use super::{Notification, Notifier, NotifyError};
use crate::events::EventSink;

const APP_NAME: &str = "Headroom";
const ICON: &str = "headroom";
const DEFAULT_ACTION: &str = "default";
const OPEN_ACTION: &str = "open";
const OPEN_LABEL: &str = "Open";
const SERVER_DEFAULT_TIMEOUT: i32 = -1;

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(
        clippy::too_many_arguments,
        reason = "mirrors the org.freedesktop.Notifications.Notify signature"
    )]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
}

pub struct DesktopNotifier {
    proxy: NotificationsProxy<'static>,
    sent: Arc<Mutex<HashSet<u32>>>,
}

impl DesktopNotifier {
    pub async fn new(conn: &Connection) -> zbus::Result<DesktopNotifier> {
        let proxy = NotificationsProxy::builder(conn)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        Ok(DesktopNotifier {
            proxy,
            sent: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub async fn forward_actions(&self, sink: Arc<dyn EventSink>) -> zbus::Result<()> {
        let mut invoked = self.proxy.receive_action_invoked().await?;
        let mut closed = self.proxy.receive_notification_closed().await?;
        loop {
            tokio::select! {
                Some(signal) = next(&mut invoked) => {
                    let args = signal.args()?;
                    if self.forget(args.id)
                        && is_open_action(&args.action_key)
                        && let Err(error) = sink.open_requested().await
                    {
                        tracing::warn!(%error, "could not emit OpenRequested");
                    }
                }
                Some(signal) = next(&mut closed) => {
                    self.forget(signal.args()?.id);
                }
                else => return Ok(()),
            }
        }
    }

    fn forget(&self, id: u32) -> bool {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id)
    }
}

#[async_trait]
impl Notifier for DesktopNotifier {
    async fn notify(&self, notification: &Notification) -> Result<(), NotifyError> {
        let id = self
            .proxy
            .notify(
                APP_NAME,
                0,
                ICON,
                &notification.title,
                &notification.body,
                &[DEFAULT_ACTION, OPEN_LABEL],
                HashMap::new(),
                SERVER_DEFAULT_TIMEOUT,
            )
            .await?;
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id);
        Ok(())
    }
}

fn is_open_action(key: &str) -> bool {
    key == DEFAULT_ACTION || key == OPEN_ACTION
}

async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

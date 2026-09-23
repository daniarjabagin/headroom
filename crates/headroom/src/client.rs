use anyhow::{Context, Result, bail};
use headroom_daemon::BusTarget;
use headroom_daemon::dbus::BUS_NAME;
use headroom_daemon::state::payload::StatePayload;
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

const NOT_RUNNING: &str =
    "the Headroom daemon is not running (start it with `systemctl --user start headroom`)";

#[zbus::proxy(
    interface = "io.github.headroom.Daemon1",
    default_service = "io.github.headroom.Daemon",
    default_path = "/io/github/headroom/Daemon",
    gen_blocking = false
)]
pub trait Daemon {
    fn get_state(&self) -> zbus::Result<String>;

    fn refresh(&self, account_id: &str) -> zbus::Result<()>;

    fn rescan(&self) -> zbus::Result<()>;

    fn set_account_label(&self, account_id: &str, label: &str) -> zbus::Result<()>;

    fn set_account_order(&self, ids: &[&str]) -> zbus::Result<()>;

    fn set_account_hidden(&self, account_id: &str, hidden: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn state_changed(&self, state: String) -> zbus::Result<()>;
}

pub async fn connect(bus: &BusTarget) -> Result<Connection> {
    headroom_daemon::dbus::connect(bus)
        .await
        .context("could not connect to the D-Bus session bus")
}

pub async fn daemon_running(conn: &Connection) -> Result<bool> {
    let bus = DBusProxy::new(conn).await?;
    let name = BusName::try_from(BUS_NAME)?;
    Ok(bus.name_has_owner(name).await?)
}

pub async fn daemon_proxy(conn: &Connection) -> Result<DaemonProxy<'static>> {
    Ok(DaemonProxy::builder(conn)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?)
}

pub async fn require_daemon(bus: &BusTarget) -> Result<DaemonProxy<'static>> {
    let conn = connect(bus).await?;
    if !daemon_running(&conn).await? {
        bail!(NOT_RUNNING);
    }
    daemon_proxy(&conn).await
}

pub async fn fetch_state(proxy: &DaemonProxy<'_>) -> Result<(String, StatePayload)> {
    let json = proxy.get_state().await.map_err(call_error)?;
    let state = parse_state(&json)?;
    Ok((json, state))
}

pub fn parse_state(json: &str) -> Result<StatePayload> {
    serde_json::from_str(json).context("the daemon sent a state payload this CLI cannot read")
}

pub fn call_error(error: zbus::Error) -> anyhow::Error {
    match error {
        zbus::Error::MethodError(_, Some(message), _) => anyhow::anyhow!(message),
        other => anyhow::Error::new(other).context("D-Bus call to the Headroom daemon failed"),
    }
}

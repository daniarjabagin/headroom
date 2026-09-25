#[cfg(target_os = "linux")]
pub mod bus;
pub mod socket;

use std::path::PathBuf;

use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use headroom_daemon::BusTarget;
use headroom_daemon::state::payload::StatePayload;
use serde_json::json;

use crate::paths::Globals;
use socket::SocketDaemon;

pub enum Transport {
    #[cfg(target_os = "linux")]
    Bus(BusTarget),
    Socket(PathBuf),
}

pub enum Daemon {
    #[cfg(target_os = "linux")]
    Bus(bus::DaemonProxy<'static>),
    Socket(SocketDaemon),
}

#[cfg(target_os = "linux")]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the socket-only build resolves a default path that can fail"
)]
pub fn transport(globals: &Globals) -> Result<Transport> {
    Ok(match &globals.socket {
        Some(path) => Transport::Socket(path.clone()),
        None => Transport::Bus(globals.bus.clone()),
    })
}

#[cfg(not(target_os = "linux"))]
pub fn transport(globals: &Globals) -> Result<Transport> {
    match &globals.socket {
        Some(path) => Ok(Transport::Socket(path.clone())),
        None => Ok(Transport::Socket(headroom_daemon::default_socket_path()?)),
    }
}

pub async fn running_daemon(globals: &Globals) -> Result<Option<Daemon>> {
    match transport(globals)? {
        #[cfg(target_os = "linux")]
        Transport::Bus(target) => Ok(bus::running_proxy(&target).await?.map(Daemon::Bus)),
        Transport::Socket(path) => Ok(SocketDaemon::connect(&path).await?.map(Daemon::Socket)),
    }
}

pub async fn require_daemon(globals: &Globals) -> Result<Daemon> {
    if let Some(daemon) = running_daemon(globals).await? {
        return Ok(daemon);
    }
    match transport(globals)? {
        #[cfg(target_os = "linux")]
        Transport::Bus(_) => Err(anyhow::anyhow!(bus::NOT_RUNNING)),
        Transport::Socket(path) => Err(socket::not_listening(&path)),
    }
}

pub async fn fetch_state(daemon: &Daemon) -> Result<(String, StatePayload)> {
    let json = daemon.get_state().await?;
    let state = parse_state(&json)?;
    Ok((json, state))
}

pub fn parse_state(json: &str) -> Result<StatePayload> {
    serde_json::from_str(json).context("the daemon sent a state payload this CLI cannot read")
}

#[cfg(target_os = "linux")]
macro_rules! over_bus {
    ($result:expr) => {
        $result.await.map_err(bus::call_error)
    };
}

impl Daemon {
    pub async fn get_state(&self) -> Result<String> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.get_state()),
            Daemon::Socket(socket) => {
                Ok(socket.call("GetState", json!([])).await?.get().to_owned())
            }
        }
    }

    pub async fn get_spend(&self, query: &str) -> Result<String> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.get_spend(query)),
            Daemon::Socket(socket) => Ok(socket
                .call("GetSpend", json!([query]))
                .await?
                .get()
                .to_owned()),
        }
    }

    pub async fn refresh(&self, account_id: &str) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.refresh(account_id)),
            Daemon::Socket(socket) => socket.command("Refresh", json!([account_id])).await,
        }
    }

    pub async fn refresh_now(&self) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.refresh_now()),
            Daemon::Socket(socket) => socket.command("RefreshNow", json!([])).await,
        }
    }

    pub async fn rescan(&self) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.rescan()),
            Daemon::Socket(socket) => socket.command("Rescan", json!([])).await,
        }
    }

    pub async fn set_account_label(&self, account_id: &str, label: &str) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.set_account_label(account_id, label)),
            Daemon::Socket(socket) => {
                socket
                    .command("SetAccountLabel", json!([account_id, label]))
                    .await
            }
        }
    }

    pub async fn set_account_order(&self, ids: &[&str]) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.set_account_order(ids)),
            Daemon::Socket(socket) => socket.command("SetAccountOrder", json!([ids])).await,
        }
    }

    pub async fn set_account_hidden(&self, account_id: &str, hidden: bool) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.set_account_hidden(account_id, hidden)),
            Daemon::Socket(socket) => {
                socket
                    .command("SetAccountHidden", json!([account_id, hidden]))
                    .await
            }
        }
    }

    pub async fn dismiss_account(&self, account_id: &str) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.dismiss_account(account_id)),
            Daemon::Socket(socket) => socket.command("DismissAccount", json!([account_id])).await,
        }
    }

    pub async fn restore_accounts(&self, provider: &str) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Daemon::Bus(proxy) => over_bus!(proxy.restore_accounts(provider)),
            Daemon::Socket(socket) => socket.command("RestoreAccounts", json!([provider])).await,
        }
    }
}

use anyhow::Result;
use futures_util::StreamExt;
use headroom_daemon::BusTarget;
use headroom_daemon::state::payload::StatePayload;
use jiff::Timestamp;
use zbus::Connection;
use zbus::proxy::OwnerChangedStream;

use super::{Printer, RERENDER_EVERY, RETRY_AFTER};
use crate::client::bus::{self, DaemonProxy, StateChanged};
use crate::client::parse_state;
use crate::render::waybar::{absent_line, state_line};

enum Ended {
    OwnerLost,
    Unreadable,
}

pub async fn run(target: &BusTarget, printer: &mut Printer) -> Result<()> {
    let conn = bus::connect(target).await?;
    let proxy = bus::daemon_proxy(&conn).await?;
    let mut owners = proxy.inner().receive_owner_changed().await?;
    loop {
        let ended = if bus::daemon_running(&conn).await? {
            follow(&conn, &proxy, printer).await?
        } else {
            Ended::OwnerLost
        };
        printer.print(absent_line())?;
        match ended {
            Ended::OwnerLost => wait_for_owner(&mut owners).await,
            Ended::Unreadable => tokio::time::sleep(RETRY_AFTER).await,
        }
    }
}

async fn wait_for_owner(owners: &mut OwnerChangedStream<'_>) {
    while let Some(owner) = owners.next().await {
        if owner.is_some() {
            return;
        }
    }
}

async fn follow(
    conn: &Connection,
    proxy: &DaemonProxy<'_>,
    printer: &mut Printer,
) -> Result<Ended> {
    let mut changes = proxy.receive_state_changed().await?;
    let mut owners = proxy.inner().receive_owner_changed().await?;
    let Some(mut state) = current_state(proxy).await else {
        return Ok(Ended::Unreadable);
    };
    let mut tick = tokio::time::interval(RERENDER_EVERY);
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            Some(signal) = changes.next() => {
                if let Some(next) = signal_state(&signal) {
                    state = next;
                }
            }
            Some(owner) = owners.next() => {
                if owner.is_none() || !bus::daemon_running(conn).await? {
                    return Ok(Ended::OwnerLost);
                }
            }
        }
        printer.print(state_line(&state, Timestamp::now()))?;
    }
}

async fn current_state(proxy: &DaemonProxy<'_>) -> Option<StatePayload> {
    let json = proxy.get_state().await.ok()?;
    parse_state(&json).ok()
}

fn signal_state(signal: &StateChanged) -> Option<StatePayload> {
    let args = signal.args().ok()?;
    match parse_state(args.state()) {
        Ok(state) => Some(state),
        Err(error) => {
            tracing::warn!(%error, "ignoring an unreadable state update");
            None
        }
    }
}

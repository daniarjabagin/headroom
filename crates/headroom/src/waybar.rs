use std::io::{self, Write};
use std::time::Duration;

use anyhow::Result;
use futures_util::StreamExt;
use headroom_daemon::state::payload::StatePayload;
use jiff::Timestamp;
use zbus::Connection;
use zbus::proxy::OwnerChangedStream;

use crate::client::{self, DaemonProxy, fetch_state, parse_state};
use crate::paths::Globals;
use crate::render::waybar::{WaybarLine, absent_line, state_line};

const RERENDER_EVERY: Duration = Duration::from_secs(60);
const RETRY_AFTER: Duration = Duration::from_secs(5);

enum Ended {
    OwnerLost,
    Unreadable,
}

struct Printer {
    last: Option<WaybarLine>,
}

impl Printer {
    fn print(&mut self, line: WaybarLine) -> Result<()> {
        if self.last.as_ref() == Some(&line) {
            return Ok(());
        }
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{}", serde_json::to_string(&line)?)?;
        stdout.flush()?;
        self.last = Some(line);
        Ok(())
    }
}

pub async fn run(globals: &Globals) -> Result<()> {
    let conn = client::connect(&globals.bus).await?;
    let proxy = client::daemon_proxy(&conn).await?;
    let mut owners = proxy.inner().receive_owner_changed().await?;
    let mut printer = Printer { last: None };
    loop {
        let ended = if client::daemon_running(&conn).await? {
            follow(&conn, &proxy, &mut printer).await?
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
    let Ok((_, mut state)) = fetch_state(proxy).await else {
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
                if owner.is_none() || !client::daemon_running(conn).await? {
                    return Ok(Ended::OwnerLost);
                }
            }
        }
        printer.print(state_line(&state, Timestamp::now()))?;
    }
}

fn signal_state(signal: &client::StateChanged) -> Option<StatePayload> {
    let args = signal.args().ok()?;
    match parse_state(args.state()) {
        Ok(state) => Some(state),
        Err(error) => {
            tracing::warn!(%error, "ignoring an unreadable state update");
            None
        }
    }
}

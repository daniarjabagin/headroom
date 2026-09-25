use std::path::Path;

use anyhow::Result;
use headroom_daemon::state::payload::StatePayload;
use jiff::Timestamp;
use serde_json::json;

use super::{Printer, RERENDER_EVERY, RETRY_AFTER};
use crate::cli::WaybarArgs;
use crate::client::parse_state;
use crate::client::socket::SocketDaemon;
use crate::render::waybar::{absent_line, state_line};

pub async fn run(path: &Path, args: &WaybarArgs, printer: &mut Printer) -> Result<()> {
    loop {
        if let Ok(Some(daemon)) = SocketDaemon::connect(path).await {
            follow(&daemon, args, printer).await?;
        }
        printer.print(absent_line())?;
        tokio::time::sleep(RETRY_AFTER).await;
    }
}

async fn follow(daemon: &SocketDaemon, args: &WaybarArgs, printer: &mut Printer) -> Result<()> {
    let Some(mut state) = subscribed_state(daemon).await else {
        return Ok(());
    };
    let mut tick = tokio::time::interval(RERENDER_EVERY);
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            change = daemon.next_state() => match change {
                Ok(Some(json)) => update(&mut state, &json),
                Ok(None) | Err(_) => return Ok(()),
            },
        }
        printer.print(state_line(&state, Timestamp::now(), args))?;
    }
}

async fn subscribed_state(daemon: &SocketDaemon) -> Option<StatePayload> {
    daemon.command("Subscribe", json!([["state"]])).await.ok()?;
    let json = daemon.call("GetState", json!([])).await.ok()?;
    parse_state(json.get()).ok()
}

fn update(state: &mut StatePayload, json: &str) {
    match parse_state(json) {
        Ok(next) => *state = next,
        Err(error) => tracing::warn!(%error, "ignoring an unreadable state update"),
    }
}

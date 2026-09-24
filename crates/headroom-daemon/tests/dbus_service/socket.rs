use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

use super::{Checked, DaemonProxy};

type Lines = tokio::io::Lines<tokio::io::BufReader<OwnedReadHalf>>;

async fn next_message(lines: &mut Lines) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let line = tokio::time::timeout(Duration::from_secs(5), lines.next_line())
        .await??
        .ok_or("socket closed")?;
    Ok(serde_json::from_str(&line)?)
}

async fn socket_call(
    write: &mut OwnedWriteHalf,
    lines: &mut Lines,
    id: u64,
    method: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request = serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method});
    write.write_all(format!("{request}\n").as_bytes()).await?;
    loop {
        let message = next_message(lines).await?;
        if message["id"] == id {
            return Ok(message);
        }
    }
}

pub async fn follows_the_daemon(path: &Path, proxy: &DaemonProxy<'_>) -> Checked {
    let (read, mut write) = UnixStream::connect(path).await?.into_split();
    let mut lines = tokio::io::BufReader::new(read).lines();
    socket_call(&mut write, &mut lines, 1, "Subscribe").await?;
    let state = socket_call(&mut write, &mut lines, 2, "GetState").await?;
    let listed = state["result"]["accounts"][0]["id"] == "codex:work";
    proxy.set_account_label("codex:work", "Socket").await?;
    loop {
        let message = next_message(&mut lines).await?;
        let label = &message["params"]["state"]["accounts"][0]["label"];
        if message["method"] == "StateChanged" && label == "Socket" {
            return Ok(listed);
        }
    }
}

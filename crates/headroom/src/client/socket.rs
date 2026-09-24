use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::Value;
use serde_json::value::RawValue;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

pub struct SocketDaemon {
    conn: Mutex<Conn>,
}

struct Conn {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    line: Vec<u8>,
    next_id: u64,
}

#[derive(Deserialize)]
struct Incoming {
    id: Option<Value>,
    method: Option<String>,
    params: Option<Box<RawValue>>,
    result: Option<Box<RawValue>>,
    error: Option<RemoteError>,
}

#[derive(Deserialize)]
struct RemoteError {
    message: String,
}

#[derive(Deserialize)]
struct StateParams {
    state: Box<RawValue>,
}

impl SocketDaemon {
    pub async fn connect(path: &Path) -> Result<Option<SocketDaemon>> {
        let stream = match UnixStream::connect(path).await {
            Ok(stream) => stream,
            Err(error) if is_absent(error.kind()) => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("could not connect to {}", path.display()));
            }
        };
        let (read, writer) = stream.into_split();
        let conn = Conn {
            reader: BufReader::new(read),
            writer,
            line: Vec::new(),
            next_id: 1,
        };
        Ok(Some(SocketDaemon {
            conn: Mutex::new(conn),
        }))
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Box<RawValue>> {
        let mut conn = self.conn.lock().await;
        let id = conn.send(method, params).await?;
        loop {
            let incoming = conn
                .receive()
                .await?
                .context("the Headroom daemon closed the connection")?;
            if incoming.id.as_ref().and_then(Value::as_u64) == Some(id) {
                return reply(incoming);
            }
        }
    }

    pub async fn command(&self, method: &str, params: Value) -> Result<()> {
        self.call(method, params).await.map(drop)
    }

    pub async fn next_state(&self) -> Result<Option<String>> {
        let mut conn = self.conn.lock().await;
        while let Some(incoming) = conn.receive().await? {
            if incoming.id.is_none() && incoming.method.as_deref() == Some("StateChanged") {
                let params = incoming.params.context("StateChanged without params")?;
                let parsed: StateParams = serde_json::from_str(params.get())?;
                return Ok(Some(parsed.state.get().to_owned()));
            }
        }
        Ok(None)
    }
}

impl Conn {
    async fn send(&mut self, method: &str, params: Value) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;
        let request =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let mut line = request.to_string();
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .await
            .context("could not send a request to the Headroom daemon")?;
        Ok(id)
    }

    async fn receive(&mut self) -> Result<Option<Incoming>> {
        let read = self
            .reader
            .read_until(b'\n', &mut self.line)
            .await
            .context("could not read from the Headroom daemon")?;
        if read == 0 || self.line.last() != Some(&b'\n') {
            return Ok(None);
        }
        let parsed = serde_json::from_slice(&self.line);
        self.line.clear();
        Ok(Some(parsed.context(
            "the Headroom daemon sent an unreadable message",
        )?))
    }
}

fn reply(incoming: Incoming) -> Result<Box<RawValue>> {
    if let Some(error) = incoming.error {
        bail!(error.message);
    }
    Ok(incoming.result.unwrap_or_else(|| RawValue::NULL.to_owned()))
}

fn is_absent(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::NotFound | ErrorKind::ConnectionRefused)
}

pub fn not_listening(path: &Path) -> anyhow::Error {
    anyhow!("the Headroom daemon is not listening on {}", path.display())
}

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use headroom_core::provider::Provider;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::task::JoinHandle;

use super::{Hub, SocketFile, accept, bind};
use crate::rescan::{self, RescanRequests};
use crate::service::Service;
use crate::testing::{CODEX, FakeProvider, Harness, account, harness, session, snapshot};

const PATIENCE: Duration = Duration::from_secs(5);
const SHORT_TEMP_ROOT: &str = "/tmp";
const LONGEST_TEMP_ROOT: usize = 48;

pub fn socket_dir() -> TempDir {
    let temp = std::env::temp_dir();
    let root = if temp.as_os_str().len() <= LONGEST_TEMP_ROOT {
        temp
    } else {
        PathBuf::from(SHORT_TEMP_ROOT)
    };
    tempfile::Builder::new()
        .prefix("hr")
        .tempdir_in(root)
        .unwrap()
}

pub struct Server {
    pub dir: TempDir,
    pub path: PathBuf,
    pub harness: Harness,
    pub hub: Arc<Hub>,
    pub file: Option<SocketFile>,
    task: JoinHandle<()>,
    _rescans: RescanRequests,
}

impl Server {
    pub async fn start() -> Server {
        let limits = snapshot(
            vec![session(20.0, "2026-09-23T12:00:00Z")],
            "2026-09-23T10:00:00Z",
        );
        let provider = FakeProvider::new(CODEX, vec![account(CODEX, "a")], limits);
        let providers: Vec<Arc<dyn Provider>> = vec![Arc::new(provider)];
        let harness = harness(providers).await;
        let dir = socket_dir();
        let path = dir.path().join("run").join("daemon.sock");
        let (listener, file) = bind(&path).unwrap();
        let (rescans, requests) = rescan::channel();
        let service = Service::new(harness.core.clone(), rescans);
        let hub = Arc::new(Hub::default());
        let task = tokio::spawn(accept(listener, service, hub.clone()));
        Server {
            dir,
            path,
            harness,
            hub,
            file: Some(file),
            task,
            _rescans: requests,
        }
    }

    pub async fn client(&self) -> Client {
        Client::connect(&self.path).await
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub struct Client {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl Client {
    pub async fn connect(path: &Path) -> Client {
        let (read, writer) = UnixStream::connect(path).await.unwrap().into_split();
        Client {
            reader: BufReader::new(read),
            writer,
        }
    }

    pub async fn send(&mut self, line: &str) {
        self.writer.write_all(line.as_bytes()).await.unwrap();
        self.writer.write_all(b"\n").await.unwrap();
    }

    pub async fn request(&mut self, id: u64, method: &str, params: Value) {
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.send(&request.to_string()).await;
    }

    pub async fn next(&mut self) -> Option<Value> {
        let mut line = String::new();
        let read = tokio::time::timeout(PATIENCE, self.reader.read_line(&mut line)).await;
        match read.unwrap() {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(serde_json::from_str(&line).unwrap()),
        }
    }

    pub async fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.request(id, method, params).await;
        loop {
            let message = self.next().await.unwrap();
            if message["id"] == id {
                return message;
            }
        }
    }

    pub async fn notification(&mut self, method: &str) -> Value {
        loop {
            let message = self.next().await.unwrap();
            if message["method"] == method {
                return message["params"].clone();
            }
        }
    }

    pub fn writer(&mut self) -> &mut OwnedWriteHalf {
        &mut self.writer
    }
}

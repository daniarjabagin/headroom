use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use serde_json::value::RawValue;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use super::dispatch::{self, Call, Command};
use super::hub::{Hub, Outbox, Subscription};
use super::protocol::{self, MAX_LINE, Request, RpcError};
use crate::service::Service;

const OUTBOX_CAPACITY: usize = 64;
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

#[derive(Clone)]
struct Context {
    service: Service,
    hub: Arc<Hub>,
    outbox: Outbox,
}

enum Line {
    Request,
    Blank,
    TooLong,
    End,
}

pub async fn accept(listener: UnixListener, service: Service, hub: Arc<Hub>) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    connections.spawn(serve(stream, service.clone(), hub.clone()));
                }
                Err(error) => {
                    tracing::warn!(%error, "could not accept a socket connection");
                    tokio::time::sleep(ACCEPT_BACKOFF).await;
                }
            },
            Some(_) = connections.join_next() => {}
        }
    }
}

async fn serve(stream: UnixStream, service: Service, hub: Arc<Hub>) {
    let (read, write) = stream.into_split();
    let (outbox, inbox) = mpsc::channel(OUTBOX_CAPACITY);
    let writer = tokio::spawn(write_lines(write, inbox));
    let context = Context {
        service,
        hub,
        outbox,
    };
    if read_requests(read, context).await == Ended::TooLong {
        writer.abort();
    }
    writer.await.ok();
}

#[derive(PartialEq, Eq)]
enum Ended {
    Closed,
    TooLong,
}

async fn read_requests(read: OwnedReadHalf, context: Context) -> Ended {
    let mut reader = BufReader::new(read);
    let mut line = Vec::new();
    let mut pending = JoinSet::new();
    let mut subscription = None;
    let mut ended = Ended::Closed;
    loop {
        while pending.try_join_next().is_some() {}
        match read_line(&mut reader, &mut line).await {
            Line::Request => {}
            Line::Blank => continue,
            Line::TooLong => {
                ended = Ended::TooLong;
                break;
            }
            Line::End => break,
        }
        let delivered = match protocol::parse_request(&line) {
            Ok(request) => handle(request, &context, &mut pending, &mut subscription).await,
            Err(rejection) => {
                send(
                    &context.outbox,
                    protocol::error_line(&rejection.id, &rejection.error),
                )
                .await
            }
        };
        if !delivered {
            break;
        }
    }
    drop(reader);
    while pending.join_next().await.is_some() {}
    ended
}

async fn read_line(reader: &mut BufReader<OwnedReadHalf>, line: &mut Vec<u8>) -> Line {
    line.clear();
    let limit = u64::try_from(MAX_LINE + 1).unwrap_or(u64::MAX);
    match (&mut *reader).take(limit).read_until(b'\n', line).await {
        Ok(0) | Err(_) => return Line::End,
        Ok(_) => {}
    }
    if line.last() == Some(&b'\n') {
        line.pop();
    } else if line.len() > MAX_LINE {
        return Line::TooLong;
    }
    if line.iter().all(u8::is_ascii_whitespace) {
        Line::Blank
    } else {
        Line::Request
    }
}

async fn handle(
    request: Request,
    context: &Context,
    pending: &mut JoinSet<()>,
    subscription: &mut Option<Subscription>,
) -> bool {
    let Request { id, method, params } = request;
    match dispatch::parse_call(&method, params) {
        Err(error) => reply(&context.outbox, id.as_ref(), Err(error)).await,
        Ok(Call::Subscribe) => {
            subscription.get_or_insert_with(|| context.hub.subscribe(context.outbox.clone()));
            reply(&context.outbox, id.as_ref(), Ok(RawValue::NULL.to_owned())).await
        }
        Ok(Call::Command(command)) => {
            pending.spawn(run_command(context.clone(), id, command));
            true
        }
    }
}

async fn run_command(context: Context, id: Option<Value>, command: Command) {
    let result = dispatch::execute(&context.service, command).await;
    reply(&context.outbox, id.as_ref(), result).await;
}

async fn reply(
    outbox: &Outbox,
    id: Option<&Value>,
    result: Result<Box<RawValue>, RpcError>,
) -> bool {
    let Some(id) = id else {
        return true;
    };
    let line = match result {
        Ok(value) => protocol::result_line(id, &value),
        Err(error) => protocol::error_line(id, &error),
    };
    send(outbox, line).await
}

async fn send(outbox: &Outbox, line: String) -> bool {
    outbox.send(Arc::from(line)).await.is_ok()
}

async fn write_lines(mut write: OwnedWriteHalf, mut inbox: mpsc::Receiver<Arc<str>>) {
    while let Some(line) = inbox.recv().await {
        let written = async {
            write.write_all(line.as_bytes()).await?;
            write.write_all(b"\n").await
        };
        if written.await.is_err() {
            return;
        }
    }
}

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

use super::protocol::MAX_LINE;
use super::test_client::{Client, Server};
use super::{Hub, bind};
use crate::error::SocketError;
use crate::events::{EventSink, publish_changes};
use crate::notify::{Notification, Notifier, NotifyError, Urgency};

fn alert() -> Notification {
    Notification {
        id: "codex:a/session/almost_out".into(),
        account_id: "codex:a".into(),
        title: "Codex — Session".into(),
        body: "Under 10% left · resets in 42m".into(),
        urgency: Urgency::Normal,
    }
}

#[tokio::test]
async fn queries_return_json_objects() {
    let server = Server::start().await;
    let mut client = server.client().await;
    let state = client.call(1, "GetState", json!([])).await;
    assert_eq!(state["jsonrpc"], "2.0");
    assert_eq!(state["result"]["version"], 1);
    assert_eq!(state["result"]["accounts"][0]["id"], "codex:a");
    let providers = client.call(2, "ListProviders", json!([])).await;
    assert_eq!(providers["result"]["providers"][0]["id"], "codex");
    let settings = client.call(3, "GetSettings", json!([])).await;
    assert_eq!(settings["result"]["refresh_interval_secs"], 300);
}

#[tokio::test]
async fn commands_reach_the_core_and_return_null() {
    let server = Server::start().await;
    let mut client = server.client().await;
    let labelled = client
        .call(1, "SetAccountLabel", json!(["codex:a", "Work"]))
        .await;
    assert_eq!(labelled["result"], Value::Null);
    let patched = client
        .call(
            2,
            "UpdateSettings",
            json!([r#"{"display":{"theme":"dark"}}"#]),
        )
        .await;
    assert_eq!(patched["result"], Value::Null);
    let state = server.harness.core.state();
    assert_eq!(state.accounts[0].label.as_deref(), Some("Work"));
    let settings = client.call(3, "GetSettings", json!([])).await;
    assert_eq!(settings["result"]["display"]["theme"], "dark");
}

#[tokio::test]
async fn errors_carry_json_rpc_codes_and_keep_the_connection() {
    let server = Server::start().await;
    let mut client = server.client().await;
    let unknown = client.call(1, "Refresh", json!(["codex:zz"])).await;
    assert_eq!(unknown["error"]["code"], -32_602);
    assert_eq!(unknown["error"]["message"], "unknown account: codex:zz");
    let invalid = client
        .call(
            2,
            "UpdateSettings",
            json!([r#"{"refresh_interval_secs":1}"#]),
        )
        .await;
    assert_eq!(invalid["error"]["code"], -32_602);
    let missing = client.call(3, "Frobnicate", json!([])).await;
    assert_eq!(missing["error"]["code"], -32_601);
    client.send("{not json").await;
    let malformed = client.next().await.unwrap();
    assert_eq!(malformed["error"]["code"], -32_700);
    assert_eq!(malformed["id"], Value::Null);
    let still_open = client.call(4, "GetState", json!([])).await;
    assert!(still_open["result"].is_object());
}

#[tokio::test]
async fn requests_without_id_get_no_response() {
    let server = Server::start().await;
    let mut client = server.client().await;
    client
        .send(r#"{"jsonrpc":"2.0","method":"Refresh","params":[""]}"#)
        .await;
    client.send("").await;
    client.request(9, "GetState", json!([])).await;
    assert_eq!(client.next().await.unwrap()["id"], 9);
}

#[tokio::test]
async fn several_requests_are_answered_without_waiting() {
    let server = Server::start().await;
    let mut client = server.client().await;
    for id in 1..=3 {
        client.request(id, "GetState", json!([])).await;
    }
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(client.next().await.unwrap()["id"].as_u64().unwrap());
    }
    ids.sort_unstable();
    assert_eq!(ids, [1, 2, 3]);
}

#[tokio::test]
async fn subscribers_receive_state_changes_and_alerts() {
    let server = Server::start().await;
    let sink: Arc<dyn EventSink> = server.hub.clone();
    let publisher = tokio::spawn(publish_changes(server.harness.core.clone(), sink));
    let mut subscriber = server.client().await;
    let mut bystander = server.client().await;
    let accepted = subscriber.call(1, "Subscribe", json!([])).await;
    assert_eq!(accepted["result"], Value::Null);
    let labelled = bystander
        .call(1, "SetAccountLabel", json!(["codex:a", "Work"]))
        .await;
    assert_eq!(labelled["result"], Value::Null);
    let changed = subscriber.notification("StateChanged").await;
    assert_eq!(changed["state"]["accounts"][0]["label"], "Work");
    server.hub.notify(&alert()).await.unwrap();
    let delivered = subscriber.notification("Alert").await;
    assert_eq!(
        delivered,
        json!({
            "id": "codex:a/session/almost_out",
            "title": "Codex — Session",
            "body": "Under 10% left · resets in 42m",
            "account_id": "codex:a",
            "urgency": "normal"
        })
    );
    server.hub.open_requested().await.unwrap();
    assert_eq!(subscriber.notification("OpenRequested").await, json!({}));
    bystander.request(2, "GetState", json!([])).await;
    let next = bystander.next().await.unwrap();
    assert_eq!(next["id"], 2, "a non-subscriber got {next}");
    publisher.abort();
}

#[tokio::test]
async fn alerts_fail_without_subscribers() {
    let server = Server::start().await;
    let mut client = server.client().await;
    client.call(1, "GetState", json!([])).await;
    let undelivered = server.hub.notify(&alert()).await;
    assert!(matches!(undelivered, Err(NotifyError::NoSubscribers)));
    client.call(2, "Subscribe", json!([])).await;
    drop(client);
    let mut watcher = server.client().await;
    watcher.call(1, "Subscribe", json!([])).await;
    server.hub.notify(&alert()).await.unwrap();
    assert_eq!(Hub::default().broadcast("x"), 0);
}

#[tokio::test]
async fn oversize_lines_close_the_connection() {
    let server = Server::start().await;
    let mut client = server.client().await;
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"GetState"}"#;
    let padded = format!("{request}{}", " ".repeat(MAX_LINE - request.len()));
    client.send(&padded).await;
    assert_eq!(client.next().await.unwrap()["id"], 1);
    let oversize = vec![b' '; MAX_LINE + 1];
    client.writer().write_all(&oversize).await.ok();
    assert_eq!(client.next().await, None);
    let fresh = server.client().await.call(2, "GetState", json!([])).await;
    assert!(fresh["result"].is_object());
}

#[tokio::test]
async fn the_socket_is_private_and_removed_on_shutdown() {
    let mut server = Server::start().await;
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&server.path), 0o600);
    assert_eq!(mode(server.path.parent().unwrap()), 0o700);
    drop(server.file.take());
    assert!(!server.path.exists());
}

#[tokio::test]
async fn a_second_daemon_is_refused_while_the_first_listens() {
    let server = Server::start().await;
    let refused = bind(&server.path);
    assert!(matches!(refused, Err(SocketError::AlreadyListening(ref p)) if *p == server.path));
    let mut client = server.client().await;
    assert!(client.call(1, "GetState", json!([])).await["result"].is_object());
}

#[tokio::test]
async fn stale_sockets_are_replaced_and_other_files_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("daemon.sock");
    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
    assert!(path.exists());
    let (listener, file) = bind(&path).unwrap();
    drop(listener);
    drop(file);
    assert!(!path.exists());
    let regular = dir.path().join("notes.txt");
    std::fs::write(&regular, "keep").unwrap();
    assert!(matches!(bind(&regular), Err(SocketError::NotASocket(_))));
    assert_eq!(std::fs::read_to_string(&regular).unwrap(), "keep");
}

#[tokio::test]
async fn a_replaced_socket_file_is_not_removed_by_the_old_owner() {
    let server = Server::start().await;
    let mut client = Client::connect(&server.path).await;
    client.call(1, "GetState", json!([])).await;
    std::fs::remove_file(&server.path).unwrap();
    let (_listener, _file) = bind(&server.path).unwrap();
    let mut server = server;
    drop(server.file.take());
    assert!(server.path.exists());
    assert!(server.dir.path().exists());
}

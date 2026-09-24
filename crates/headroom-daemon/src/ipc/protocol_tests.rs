use serde_json::json;

use super::*;
use crate::notify::Urgency;

fn parsed(line: &str) -> Value {
    serde_json::from_str(line).unwrap()
}

fn rejected(line: &str) -> (Value, i32) {
    let rejection = parse_request(line.as_bytes()).unwrap_err();
    (rejection.id, rejection.error.code.code())
}

#[test]
fn requests_keep_their_id_method_and_positional_params() {
    let request =
        parse_request(br#"{"jsonrpc":"2.0","id":7,"method":"Refresh","params":["codex:a"]}"#)
            .unwrap();
    assert_eq!(request.id, Some(json!(7)));
    assert_eq!(request.method, "Refresh");
    assert_eq!(request.params, [json!("codex:a")]);
    let bare = parse_request(br#"{"jsonrpc":"2.0","id":"x","method":"GetState"}"#).unwrap();
    assert_eq!(bare.id, Some(json!("x")));
    assert!(bare.params.is_empty());
    let notification = parse_request(br#"{"jsonrpc":"2.0","method":"RefreshNow"}"#).unwrap();
    assert_eq!(notification.id, None);
    let null_id = parse_request(br#"{"jsonrpc":"2.0","id":null,"method":"Rescan"}"#).unwrap();
    assert_eq!(null_id.id, Some(Value::Null));
}

#[test]
fn malformed_requests_are_rejected_with_json_rpc_codes() {
    let cases = [
        ("{not json", Value::Null, -32_700),
        (r#"[{"jsonrpc":"2.0"}]"#, Value::Null, -32_600),
        (r#""GetState""#, Value::Null, -32_600),
        (
            r#"{"jsonrpc":"1.0","id":1,"method":"GetState"}"#,
            json!(1),
            -32_600,
        ),
        (r#"{"id":2,"method":"GetState"}"#, json!(2), -32_600),
        (r#"{"jsonrpc":"2.0","id":3,"method":5}"#, json!(3), -32_600),
        (
            r#"{"jsonrpc":"2.0","id":[4],"method":"GetState"}"#,
            Value::Null,
            -32_600,
        ),
        (
            r#"{"jsonrpc":"2.0","id":5,"method":"Refresh","params":{"a":1}}"#,
            json!(5),
            -32_602,
        ),
    ];
    for (line, id, code) in cases {
        assert_eq!(rejected(line), (id, code), "{line}");
    }
}

#[test]
fn invalid_utf8_is_a_parse_error() {
    let rejection = parse_request(b"{\"jsonrpc\":\"2.0\",\"method\":\"\xff\"}").unwrap_err();
    assert_eq!(rejection.error.code, ErrorCode::Parse);
}

#[test]
fn responses_echo_the_id_and_embed_the_result() {
    let result = RawValue::from_string(r#"{"version":1}"#.to_owned()).unwrap();
    assert_eq!(
        parsed(&result_line(&json!("a"), &result)),
        json!({"jsonrpc": "2.0", "id": "a", "result": {"version": 1}})
    );
    let error = RpcError::new(ErrorCode::InvalidParams, "unknown account: \"x\"\n");
    assert_eq!(
        parsed(&error_line(&json!(8), &error)),
        json!({"jsonrpc": "2.0", "id": 8, "error": {"code": -32602, "message": "unknown account: \"x\"\n"}})
    );
}

#[test]
fn notifications_have_no_id_and_fit_on_one_line() {
    let state = parsed(&state_changed_line(r#"{"version":1,"accounts":[]}"#));
    assert_eq!(
        state,
        json!({"jsonrpc": "2.0", "method": "StateChanged", "params": {"state": {"version": 1, "accounts": []}}})
    );
    assert_eq!(
        parsed(&open_requested_line()),
        json!({"jsonrpc": "2.0", "method": "OpenRequested", "params": {}})
    );
    let alert = Notification {
        id: "codex:a/session/almost_out".into(),
        account_id: "codex:a".into(),
        title: "Codex · Work — Session".into(),
        body: "Under 10% left\nsoon".into(),
        urgency: Urgency::Critical,
    };
    let line = alert_line(&alert);
    assert!(!line.contains('\n'));
    assert_eq!(
        parsed(&line),
        json!({"jsonrpc": "2.0", "method": "Alert", "params": {
            "id": "codex:a/session/almost_out",
            "title": "Codex · Work — Session",
            "body": "Under 10% left\nsoon",
            "account_id": "codex:a",
            "urgency": "critical"
        }})
    );
}

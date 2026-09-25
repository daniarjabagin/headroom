use serde_json::json;

use super::*;
use crate::rescan;
use crate::testing::harness;
use crate::update::{self, CheckOutcome};

fn call(method: &str, params: Value) -> Result<Call, RpcError> {
    let Value::Array(params) = params else {
        panic!("params must be an array");
    };
    parse_call(method, params)
}

fn command(method: &str, params: Value) -> Command {
    match call(method, params).unwrap() {
        Call::Command(command) => command,
        Call::Subscribe(_) => panic!("{method} parsed as Subscribe"),
    }
}

fn error_code(method: &str, params: Value) -> ErrorCode {
    call(method, params).unwrap_err().code
}

#[test]
fn every_method_takes_its_d_bus_arguments_in_order() {
    let cases = [
        ("GetState", json!([]), Command::GetState),
        ("ListProviders", json!([]), Command::ListProviders),
        ("GetSettings", json!([]), Command::GetSettings),
        (
            "GetSpend",
            json!([r#"{"period":"7d","by":"model"}"#]),
            Command::GetSpend(r#"{"period":"7d","by":"model"}"#.into()),
        ),
        (
            "Refresh",
            json!(["codex:a"]),
            Command::Refresh("codex:a".into()),
        ),
        ("RefreshNow", json!([]), Command::RefreshNow),
        ("Rescan", json!([]), Command::Rescan),
        ("CheckForUpdates", json!([]), Command::CheckForUpdates),
        ("GetDiagnostics", json!([]), Command::GetDiagnostics),
        (
            "SetSettings",
            json!(["{}"]),
            Command::SetSettings("{}".into()),
        ),
        (
            "UpdateSettings",
            json!(["{}"]),
            Command::UpdateSettings("{}".into()),
        ),
        ("ResetSettings", json!([]), Command::ResetSettings),
        (
            "SetAccountLabel",
            json!(["codex:a", "Work"]),
            Command::SetAccountLabel("codex:a".into(), "Work".into()),
        ),
        (
            "SetAccountOrder",
            json!([["codex:b", "codex:a"]]),
            Command::SetAccountOrder(vec!["codex:b".into(), "codex:a".into()]),
        ),
        (
            "SetAccountHidden",
            json!(["codex:a", true]),
            Command::SetAccountHidden("codex:a".into(), true),
        ),
        (
            "DismissAccount",
            json!(["codex:a"]),
            Command::DismissAccount("codex:a".into()),
        ),
        (
            "RestoreAccounts",
            json!([""]),
            Command::RestoreAccounts(String::new()),
        ),
    ];
    for (method, params, expected) in cases {
        assert_eq!(command(method, params), expected, "{method}");
    }
}

#[test]
fn subscribe_takes_an_optional_topic_list() {
    let only_state = Topics::from_list(&[Topic::State]);
    let cases = [
        (json!([]), Topics::ALL),
        (json!([[]]), Topics::ALL),
        (json!([["state", "alerts", "open"]]), Topics::ALL),
        (json!([["state"]]), only_state),
        (json!([["state", "state"]]), only_state),
    ];
    for (params, expected) in cases {
        let parsed = call("Subscribe", params.clone()).unwrap();
        assert_eq!(parsed, Call::Subscribe(expected), "{params}");
    }
    assert!(only_state.contains(Topic::State));
    assert!(!only_state.contains(Topic::Alerts));
    assert!(!only_state.contains(Topic::Open));
    let unknown = call("Subscribe", json!([["state", "gossip"]])).unwrap_err();
    assert_eq!(unknown.code, ErrorCode::InvalidParams);
    assert_eq!(unknown.message, "unknown topic \"gossip\"");
}

#[test]
fn wrong_arguments_are_invalid_params() {
    let cases = [
        ("Refresh", json!([])),
        ("Refresh", json!([1])),
        ("Refresh", json!(["a", "b"])),
        ("GetState", json!([null])),
        ("GetDiagnostics", json!([true])),
        ("Subscribe", json!([true])),
        ("Subscribe", json!([[1]])),
        ("Subscribe", json!([["state"], ["alerts"]])),
        ("SetAccountHidden", json!(["codex:a", "yes"])),
        ("SetAccountOrder", json!(["codex:a"])),
        ("SetAccountOrder", json!([["codex:a", 2]])),
        ("SetAccountLabel", json!(["codex:a"])),
        ("GetSpend", json!([])),
        ("GetSpend", json!([{"period": "7d"}])),
    ];
    for (method, params) in cases {
        assert_eq!(
            error_code(method, params.clone()),
            ErrorCode::InvalidParams,
            "{method} {params}"
        );
    }
}

#[test]
fn unknown_methods_are_not_found() {
    assert_eq!(
        error_code("get_state", json!([])),
        ErrorCode::MethodNotFound
    );
    assert_eq!(
        error_code("Frobnicate", json!([1])),
        ErrorCode::MethodNotFound
    );
}

#[test]
fn command_errors_keep_the_d_bus_classification() {
    let unknown = rpc_error(&CommandError::UnknownAccount("codex:zz".into()));
    assert_eq!(unknown.code, ErrorCode::InvalidParams);
    assert_eq!(unknown.message, "unknown account: codex:zz");
    assert_eq!(rpc_error(&CommandError::Stopping).code, ErrorCode::Internal);
    let unsupported = rpc_error(&CommandError::UpdateChecksUnavailable);
    assert_eq!(unsupported.code, ErrorCode::Internal);
}

#[tokio::test]
async fn update_checks_return_the_outcome_as_an_object() {
    let harness = harness(Vec::new()).await;
    let (rescans, _rescan_requests) = rescan::channel();
    let (checks, mut requests) = update::channel();
    tokio::spawn(async move {
        while let Some(waiters) = requests.next().await {
            for waiter in waiters {
                waiter.send(CheckOutcome::disabled()).ok();
            }
        }
    });
    let service = Service::new(harness.core.clone(), rescans).with_update_checks(checks);
    let result = execute(&service, Command::CheckForUpdates).await.unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(result.get()).unwrap(),
        json!({"status": "disabled", "checked_at": null, "version": null})
    );
}

#[tokio::test]
async fn diagnostics_return_the_report_as_an_object() {
    let harness = harness(Vec::new()).await;
    let (rescans, _rescan_requests) = rescan::channel();
    let service = Service::new(harness.core.clone(), rescans);
    let result = execute(&service, Command::GetDiagnostics).await.unwrap();
    let report: Value = serde_json::from_str(result.get()).unwrap();
    assert_eq!(report["app_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["log_level"], "info");
    assert_eq!(report["log_level_source"], "settings");
    assert!(report["text"].as_str().unwrap().starts_with("Headroom "));
}

#[tokio::test]
async fn spend_returns_an_object_and_rejects_bad_queries_as_invalid_params() {
    let harness = harness(Vec::new()).await;
    let (rescans, _rescan_requests) = rescan::channel();
    let service = Service::new(harness.core.clone(), rescans);
    let query = r#"{"period":"today","by":"provider"}"#.to_owned();
    let result = execute(&service, Command::GetSpend(query)).await.unwrap();
    let report = serde_json::from_str::<Value>(result.get()).unwrap();
    assert_eq!(report["since"], "2026-09-23");
    assert_eq!(report["rows"], json!([]));
    assert_eq!(report["total"]["key"], Value::Null);
    let bad = [
        r#"{"period":"week","by":"model"}"#,
        r#"{"period":"7d"}"#,
        r#"{"period":"7d","by":"model","extra":1}"#,
        r#"{"since":"2026-09-20","until":"2026-09-19","by":"day"}"#,
        r#"{"period":"7d","by":"model","provider":"nope"}"#,
    ];
    for query in bad {
        let error = execute(&service, Command::GetSpend(query.into()))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidParams, "{query}");
    }
}

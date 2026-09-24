use serde_json::json;

use super::*;

fn call(method: &str, params: Value) -> Result<Call, RpcError> {
    let Value::Array(params) = params else {
        panic!("params must be an array");
    };
    parse_call(method, params)
}

fn command(method: &str, params: Value) -> Command {
    match call(method, params).unwrap() {
        Call::Command(command) => command,
        Call::Subscribe => panic!("{method} parsed as Subscribe"),
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
            "Refresh",
            json!(["codex:a"]),
            Command::Refresh("codex:a".into()),
        ),
        ("RefreshNow", json!([]), Command::RefreshNow),
        ("Rescan", json!([]), Command::Rescan),
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
    assert_eq!(call("Subscribe", json!([])).unwrap(), Call::Subscribe);
}

#[test]
fn wrong_arguments_are_invalid_params() {
    let cases = [
        ("Refresh", json!([])),
        ("Refresh", json!([1])),
        ("Refresh", json!(["a", "b"])),
        ("GetState", json!([null])),
        ("Subscribe", json!([true])),
        ("SetAccountHidden", json!(["codex:a", "yes"])),
        ("SetAccountOrder", json!(["codex:a"])),
        ("SetAccountOrder", json!([["codex:a", 2]])),
        ("SetAccountLabel", json!(["codex:a"])),
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
}

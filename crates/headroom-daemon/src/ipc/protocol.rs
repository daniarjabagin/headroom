use serde_json::value::RawValue;
use serde_json::{Map, Value};

use crate::notify::Notification;

pub const MAX_LINE: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Parse,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    Internal,
}

impl ErrorCode {
    #[must_use]
    pub fn code(self) -> i32 {
        match self {
            ErrorCode::Parse => -32_700,
            ErrorCode::InvalidRequest => -32_600,
            ErrorCode::MethodNotFound => -32_601,
            ErrorCode::InvalidParams => -32_602,
            ErrorCode::Internal => -32_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: ErrorCode,
    pub message: String,
}

impl RpcError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> RpcError {
        RpcError {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: Option<Value>,
    pub method: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rejection {
    pub id: Option<Value>,
    pub error: RpcError,
}

pub fn parse_request(line: &[u8]) -> Result<Request, Rejection> {
    let value: Value = serde_json::from_slice(line)
        .map_err(|error| reject(Some(Value::Null), ErrorCode::Parse, error.to_string()))?;
    let Value::Object(mut object) = value else {
        return Err(invalid(Value::Null, "a request must be a JSON object"));
    };
    let id = request_id(&mut object)?;
    let echo = id.clone().unwrap_or(Value::Null);
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(invalid(echo, "jsonrpc must be \"2.0\""));
    }
    let Some(Value::String(method)) = object.remove("method") else {
        return Err(invalid(echo, "method must be a string"));
    };
    let params = match object.remove("params") {
        None => Vec::new(),
        Some(Value::Array(params)) => params,
        Some(_) => {
            return Err(reject(
                id,
                ErrorCode::InvalidParams,
                "params must be an array",
            ));
        }
    };
    Ok(Request { id, method, params })
}

fn request_id(object: &mut Map<String, Value>) -> Result<Option<Value>, Rejection> {
    match object.remove("id") {
        None => Ok(None),
        Some(id @ (Value::Null | Value::Number(_) | Value::String(_))) => Ok(Some(id)),
        Some(_) => Err(invalid(
            Value::Null,
            "id must be a string, a number or null",
        )),
    }
}

fn invalid(id: Value, message: &str) -> Rejection {
    reject(Some(id), ErrorCode::InvalidRequest, message)
}

fn reject(id: Option<Value>, code: ErrorCode, message: impl Into<String>) -> Rejection {
    Rejection {
        id,
        error: RpcError::new(code, message),
    }
}

#[must_use]
pub fn result_line(id: &Value, result: &RawValue) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{}}}"#, result.get())
}

#[must_use]
pub fn error_line(id: &Value, error: &RpcError) -> String {
    let message = Value::String(error.message.clone());
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":{},"message":{message}}}}}"#,
        error.code.code()
    )
}

#[must_use]
pub fn state_changed_line(state: &str) -> String {
    notification_line("StateChanged", &format!(r#"{{"state":{state}}}"#))
}

#[must_use]
pub fn open_requested_line() -> String {
    notification_line("OpenRequested", "{}")
}

#[must_use]
pub fn alert_line(alert: &Notification) -> String {
    let params = serde_json::json!({
        "id": alert.id,
        "title": alert.title,
        "body": alert.body,
        "account_id": alert.account_id,
        "urgency": alert.urgency.as_str(),
    });
    notification_line("Alert", &params.to_string())
}

fn notification_line(method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","method":"{method}","params":{params}}}"#)
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

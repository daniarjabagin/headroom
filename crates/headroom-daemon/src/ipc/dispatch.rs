use serde_json::Value;
use serde_json::value::{RawValue, to_raw_value};

use super::hub::{Topic, Topics};
use super::protocol::{ErrorCode, RpcError};
use crate::error::CommandError;
use crate::service::Service;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Subscribe(Topics),
    Command(Command),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    GetState,
    ListProviders,
    GetSettings,
    Refresh(String),
    RefreshNow,
    Rescan,
    SetSettings(String),
    UpdateSettings(String),
    SetAccountLabel(String, String),
    SetAccountOrder(Vec<String>),
    SetAccountHidden(String, bool),
    DismissAccount(String),
    RestoreAccounts(String),
}

pub fn parse_call(method: &str, params: Vec<Value>) -> Result<Call, RpcError> {
    let mut args = Args::new(params);
    let call = match method {
        "Subscribe" => Call::Subscribe(args.topics()?),
        other => Call::Command(parse_command(other, &mut args)?),
    };
    args.finish()?;
    Ok(call)
}

fn parse_command(method: &str, args: &mut Args) -> Result<Command, RpcError> {
    Ok(match method {
        "GetState" => Command::GetState,
        "ListProviders" => Command::ListProviders,
        "GetSettings" => Command::GetSettings,
        "Refresh" => Command::Refresh(args.string()?),
        "RefreshNow" => Command::RefreshNow,
        "Rescan" => Command::Rescan,
        "SetSettings" => Command::SetSettings(args.string()?),
        "UpdateSettings" => Command::UpdateSettings(args.string()?),
        "SetAccountLabel" => Command::SetAccountLabel(args.string()?, args.string()?),
        "SetAccountOrder" => Command::SetAccountOrder(args.strings()?),
        "SetAccountHidden" => Command::SetAccountHidden(args.string()?, args.boolean()?),
        "DismissAccount" => Command::DismissAccount(args.string()?),
        "RestoreAccounts" => Command::RestoreAccounts(args.string()?),
        unknown => {
            return Err(RpcError::new(
                ErrorCode::MethodNotFound,
                format!("unknown method {unknown}"),
            ));
        }
    })
}

struct Args {
    values: std::vec::IntoIter<Value>,
    position: usize,
}

impl Args {
    fn new(params: Vec<Value>) -> Args {
        Args {
            values: params.into_iter(),
            position: 0,
        }
    }

    fn next(&mut self, kind: &str) -> Result<Value, RpcError> {
        self.position += 1;
        self.values
            .next()
            .ok_or_else(|| invalid(format!("argument {} ({kind}) is missing", self.position)))
    }

    fn string(&mut self) -> Result<String, RpcError> {
        match self.next("a string")? {
            Value::String(text) => Ok(text),
            _ => Err(self.wrong_type("a string")),
        }
    }

    fn boolean(&mut self) -> Result<bool, RpcError> {
        match self.next("a boolean")? {
            Value::Bool(flag) => Ok(flag),
            _ => Err(self.wrong_type("a boolean")),
        }
    }

    fn strings(&mut self) -> Result<Vec<String>, RpcError> {
        let Value::Array(items) = self.next("an array of strings")? else {
            return Err(self.wrong_type("an array of strings"));
        };
        items
            .into_iter()
            .map(|item| match item {
                Value::String(text) => Ok(text),
                _ => Err(self.wrong_type("an array of strings")),
            })
            .collect()
    }

    fn topics(&mut self) -> Result<Topics, RpcError> {
        if self.values.len() == 0 {
            return Ok(Topics::ALL);
        }
        let topics = self
            .strings()?
            .iter()
            .map(|name| {
                Topic::parse(name).ok_or_else(|| invalid(format!("unknown topic {name:?}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Topics::from_list(&topics))
    }

    fn wrong_type(&self, kind: &str) -> RpcError {
        invalid(format!("argument {} must be {kind}", self.position))
    }

    fn finish(mut self) -> Result<(), RpcError> {
        match self.values.next() {
            None => Ok(()),
            Some(_) => Err(invalid(format!(
                "expected {} argument(s), got more",
                self.position
            ))),
        }
    }
}

fn invalid(message: String) -> RpcError {
    RpcError::new(ErrorCode::InvalidParams, message)
}

pub async fn execute(service: &Service, command: Command) -> Result<Box<RawValue>, RpcError> {
    run(service, command)
        .await
        .map_err(|error| rpc_error(&error))
}

async fn run(service: &Service, command: Command) -> Result<Box<RawValue>, CommandError> {
    let core = service.core();
    match command {
        Command::GetState => return Ok(to_raw_value(&core.state())?),
        Command::ListProviders => return Ok(to_raw_value(&core.catalog.payload())?),
        Command::GetSettings => return Ok(RawValue::from_string(core.settings_json()?)?),
        Command::Refresh(id) => service.refresh(&id)?,
        Command::RefreshNow => core.refresh_now(),
        Command::Rescan => service.rescan().await?,
        Command::SetSettings(json) => core.set_settings(&json).await?,
        Command::UpdateSettings(patch) => core.update_settings(&patch).await?,
        Command::SetAccountLabel(id, label) => core.set_account_label(&id, &label).await?,
        Command::SetAccountOrder(ids) => core.set_account_order(&ids).await?,
        Command::SetAccountHidden(id, hidden) => core.set_account_hidden(&id, hidden).await?,
        Command::DismissAccount(id) => service.dismiss_account(&id).await?,
        Command::RestoreAccounts(provider) => service.restore_accounts(&provider).await?,
    }
    Ok(RawValue::NULL.to_owned())
}

fn rpc_error(error: &CommandError) -> RpcError {
    let code = if error.is_invalid_argument() {
        ErrorCode::InvalidParams
    } else {
        ErrorCode::Internal
    };
    RpcError::new(code, error.to_string())
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;

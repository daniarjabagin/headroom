use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Recovery {
    Retry,
    SignIn {
        #[serde(default)]
        account_id: Option<String>,
    },
    CliLogin {
        #[serde(default)]
        command: String,
        #[serde(default)]
        account_id: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RecoveryField {
    #[default]
    Unreported,
    Wait,
    Offered(Recovery),
}

impl<'de> Deserialize<'de> for RecoveryField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Value::deserialize(deserializer)? {
            Value::Null => RecoveryField::Wait,
            value => {
                RecoveryField::Offered(serde_json::from_value(value).unwrap_or(Recovery::Unknown))
            }
        })
    }
}

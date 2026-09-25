use serde::Deserialize;

use crate::i18n::{Lang, fill};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    #[error("unreadable provider list from the Headroom service: {0}")]
    Unreadable(String),
    #[error("the Headroom service lists providers in version {0}, expected {SCHEMA_VERSION}")]
    Version(u32),
}

impl RegistryError {
    #[must_use]
    pub fn message(&self, lang: Lang) -> String {
        match self {
            RegistryError::Unreadable(reason) => fill(
                lang.tr("Unreadable provider list from the Headroom service: {reason}"),
                &[("reason", reason)],
            ),
            RegistryError::Version(actual) => fill(
                lang.tr(
                    "Headroom service lists providers in version {actual}, expected {expected}",
                ),
                &[
                    ("actual", &actual.to_string()),
                    ("expected", &SCHEMA_VERSION.to_string()),
                ],
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddMethod {
    CliLogin {
        program: Option<String>,
    },
    ApiKey {
        label: Option<String>,
        console_url: Option<String>,
        hint: Option<String>,
    },
    AutoDetect {
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderLinks {
    pub status: Option<String>,
    pub dashboard: Option<String>,
    pub usage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub methods: Vec<AddMethod>,
    pub links: ProviderLinks,
}

#[derive(Debug, Deserialize)]
struct RawRegistry {
    version: u32,
    #[serde(default)]
    providers: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RawProvider {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    add_account: Vec<RawMethod>,
    #[serde(default)]
    links: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct RawLinks {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    dashboard: Option<String>,
    #[serde(default)]
    usage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMethod {
    kind: String,
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    console_url: Option<String>,
    #[serde(default)]
    hint: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

fn text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn https_url(value: Option<String>) -> Option<String> {
    text(value).filter(|url| url.starts_with("https://") && !url.contains(char::is_whitespace))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn method(raw: RawMethod) -> Option<AddMethod> {
    match raw.kind.as_str() {
        "cli_login" => Some(AddMethod::CliLogin {
            program: text(raw.program),
        }),
        "api_key" => Some(AddMethod::ApiKey {
            label: text(raw.label),
            console_url: https_url(raw.console_url),
            hint: text(raw.hint),
        }),
        "auto_detect" => Some(AddMethod::AutoDetect {
            reason: text(raw.reason),
        }),
        _ => None,
    }
}

fn links(value: Option<serde_json::Value>) -> ProviderLinks {
    let raw: RawLinks = value
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    ProviderLinks {
        status: https_url(raw.status),
        dashboard: https_url(raw.dashboard),
        usage: https_url(raw.usage),
    }
}

fn provider(value: serde_json::Value) -> Option<ProviderInfo> {
    let raw: RawProvider = serde_json::from_value(value).ok()?;
    if !valid_id(&raw.id) {
        return None;
    }
    let methods: Vec<AddMethod> = raw.add_account.into_iter().filter_map(method).collect();
    if methods.is_empty() {
        return None;
    }
    Some(ProviderInfo {
        display_name: text(raw.display_name).unwrap_or_else(|| raw.id.clone()),
        id: raw.id,
        methods,
        links: links(raw.links),
    })
}

pub fn parse_providers(json: &str) -> Result<Vec<ProviderInfo>, RegistryError> {
    let raw: RawRegistry =
        serde_json::from_str(json).map_err(|error| RegistryError::Unreadable(error.to_string()))?;
    if raw.version != SCHEMA_VERSION {
        return Err(RegistryError::Version(raw.version));
    }
    let mut providers: Vec<ProviderInfo> = Vec::new();
    for info in raw.providers.into_iter().filter_map(provider) {
        if providers.iter().all(|known| known.id != info.id) {
            providers.push(info);
        }
    }
    Ok(providers)
}

#[must_use]
pub fn method_summary(lang: Lang, method: &AddMethod) -> String {
    match method {
        AddMethod::CliLogin {
            program: Some(program),
        } => fill(lang.tr("Sign in with {program}"), &[("program", program)]),
        AddMethod::CliLogin { program: None } => lang.tr("Sign in").to_owned(),
        AddMethod::ApiKey { .. } => lang.tr("API key").to_owned(),
        AddMethod::AutoDetect { .. } => lang.tr("Detected automatically").to_owned(),
    }
}

#[must_use]
pub fn provider_summary(lang: Lang, provider: &ProviderInfo) -> String {
    provider
        .methods
        .iter()
        .map(|method| method_summary(lang, method))
        .collect::<Vec<_>>()
        .join(" · ")
}

#[must_use]
pub fn add_account_args(provider_id: &str, method: &AddMethod, label: &str) -> Vec<String> {
    let mut args: Vec<String> = vec!["accounts".into(), "add".into(), provider_id.into()];
    if matches!(method, AddMethod::ApiKey { .. }) {
        args.push("--api-key-stdin".into());
    }
    let label = label.trim();
    if !label.is_empty() {
        args.push(format!("--label={label}"));
    }
    args.extend(["--progress".into(), "json".into()]);
    args
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;

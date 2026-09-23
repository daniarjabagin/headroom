use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    Supplement,
    LiteLlm,
    ModelsDev,
}

impl fmt::Display for PriceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PriceSource::Supplement => "supplement",
            PriceSource::LiteLlm => "LiteLLM",
            PriceSource::ModelsDev => "models.dev",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vendor {
    #[serde(rename = "openai")]
    OpenAi,
    Anthropic,
}

impl Vendor {
    pub(crate) fn from_id(id: &str) -> Option<Vendor> {
        match id {
            "openai" => Some(Vendor::OpenAi),
            "anthropic" => Some(Vendor::Anthropic),
            _ => None,
        }
    }

    pub(crate) const ALL: [Vendor; 2] = [Vendor::OpenAi, Vendor::Anthropic];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Vendor::OpenAi => "openai",
            Vendor::Anthropic => "anthropic",
        }
    }
}

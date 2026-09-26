use std::sync::Arc;

use headroom_core::account::ProviderId;
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor, ProviderLinks};
use headroom_core::provider::Provider;
use jiff::SignedDuration;
use serde::Serialize;

pub const PROVIDERS_VERSION: u32 = 1;

/// Display facts about every compiled-in provider, whether or not it could start.
#[derive(Debug, Clone, Default)]
pub struct ProviderCatalog {
    descriptors: Vec<&'static ProviderDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvidersPayload {
    pub version: u32,
    pub providers: Vec<ProviderView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderView {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub add_account: Vec<AddAccountView>,
    pub multi_account: bool,
    pub local_usage: bool,
    pub links: LinksView,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LinksView {
    pub status: Option<&'static str>,
    pub dashboard: Option<&'static str>,
    pub usage: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddAccountView {
    CliLogin {
        program: &'static str,
    },
    ApiKey {
        label: &'static str,
        console_url: &'static str,
        hint: &'static str,
    },
    AutoDetect {
        reason: &'static str,
    },
}

impl ProviderCatalog {
    #[must_use]
    pub fn new(
        descriptors: impl IntoIterator<Item = &'static ProviderDescriptor>,
    ) -> ProviderCatalog {
        ProviderCatalog {
            descriptors: descriptors.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn of_providers(providers: &[Arc<dyn Provider>]) -> ProviderCatalog {
        ProviderCatalog::new(providers.iter().map(|provider| provider.descriptor()))
    }

    /// The registry's display name, or the id itself for a provider this build does not know.
    #[must_use]
    pub fn display_name<'a>(&'a self, id: &'a ProviderId) -> &'a str {
        self.descriptors
            .iter()
            .find(|descriptor| &descriptor.id == id)
            .map_or(id.as_str(), |descriptor| descriptor.display_name)
    }

    #[must_use]
    pub fn descriptor(&self, id: &ProviderId) -> Option<&'static ProviderDescriptor> {
        self.descriptors
            .iter()
            .copied()
            .find(|descriptor| &descriptor.id == id)
    }

    #[must_use]
    pub fn min_poll_interval(&self, id: &ProviderId) -> Option<SignedDuration> {
        self.descriptor(id)
            .and_then(|descriptor| descriptor.min_poll_interval)
    }

    /// Position in the registry, which orders per-provider lists; unknown providers come last.
    #[must_use]
    pub fn rank(&self, id: &ProviderId) -> usize {
        self.descriptors
            .iter()
            .position(|descriptor| &descriptor.id == id)
            .unwrap_or(self.descriptors.len())
    }

    #[must_use]
    pub fn payload(&self) -> ProvidersPayload {
        ProvidersPayload {
            version: PROVIDERS_VERSION,
            providers: self
                .descriptors
                .iter()
                .copied()
                .map(provider_view)
                .collect(),
        }
    }
}

fn provider_view(descriptor: &'static ProviderDescriptor) -> ProviderView {
    ProviderView {
        id: descriptor.id.clone(),
        display_name: descriptor.display_name,
        add_account: descriptor.add_account.iter().map(method_view).collect(),
        multi_account: descriptor.multi_account,
        local_usage: descriptor.local_usage,
        links: LinksView::from(descriptor.links),
    }
}

impl From<ProviderLinks> for LinksView {
    fn from(links: ProviderLinks) -> LinksView {
        LinksView {
            status: links.status,
            dashboard: links.dashboard,
            usage: links.usage,
        }
    }
}

fn method_view(method: &'static AddAccountMethod) -> AddAccountView {
    match method {
        AddAccountMethod::CliLogin(login) => AddAccountView::CliLogin {
            program: login.program,
        },
        AddAccountMethod::ApiKey(prompt) => AddAccountView::ApiKey {
            label: prompt.label,
            console_url: prompt.console_url,
            hint: prompt.hint,
        },
        AddAccountMethod::AutoDetect { reason } => AddAccountView::AutoDetect { reason },
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::descriptor::ApiKeyPrompt;
    use serde_json::json;

    use super::*;
    use crate::testing::{CLAUDE, CODEX, CODEX_DESCRIPTOR};

    static KEYED: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("keyed"),
        display_name: "Keyed",
        add_account: &[
            AddAccountMethod::ApiKey(ApiKeyPrompt {
                label: "API key",
                console_url: "https://keyed.example/keys",
                hint: "Starts with kd-",
            }),
            AddAccountMethod::AutoDetect {
                reason: "reads KEYED_API_KEY",
            },
        ],
        multi_account: true,
        local_usage: false,
        min_poll_interval: None,
        links: ProviderLinks {
            status: Some("https://status.keyed.example"),
            dashboard: None,
            usage: Some("https://keyed.example/usage"),
        },
    };

    #[test]
    fn names_come_from_descriptors_with_the_id_as_fallback() {
        let catalog = ProviderCatalog::new([&CODEX_DESCRIPTOR]);
        assert_eq!(catalog.display_name(&CODEX), "Codex");
        assert_eq!(catalog.display_name(&CLAUDE), "claude");
    }

    #[test]
    fn rank_follows_registry_order_with_unknown_ids_last() {
        let catalog = ProviderCatalog::new([&CODEX_DESCRIPTOR, &KEYED]);
        assert_eq!(catalog.rank(&CODEX), 0);
        assert_eq!(catalog.rank(&KEYED.id), 1);
        assert_eq!(catalog.rank(&CLAUDE), 2);
    }

    #[test]
    fn payload_lists_methods_in_order_without_internal_fields() {
        let catalog = ProviderCatalog::new([&CODEX_DESCRIPTOR, &KEYED]);
        let json = serde_json::to_value(catalog.payload()).unwrap();
        let expected = json!({
            "version": 1,
            "providers": [
                {
                    "id": "codex",
                    "display_name": "Codex",
                    "add_account": [{ "kind": "cli_login", "program": "codex" }],
                    "multi_account": true,
                    "local_usage": true,
                    "links": {
                        "status": "https://status.openai.com",
                        "dashboard": null,
                        "usage": null
                    }
                },
                {
                    "id": "keyed",
                    "display_name": "Keyed",
                    "add_account": [
                        {
                            "kind": "api_key",
                            "label": "API key",
                            "console_url": "https://keyed.example/keys",
                            "hint": "Starts with kd-"
                        },
                        { "kind": "auto_detect", "reason": "reads KEYED_API_KEY" }
                    ],
                    "multi_account": true,
                    "local_usage": false,
                    "links": {
                        "status": "https://status.keyed.example",
                        "dashboard": null,
                        "usage": "https://keyed.example/usage"
                    }
                }
            ]
        });
        assert_eq!(json, expected);
    }
}

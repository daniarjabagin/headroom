use headroom_daemon::catalog::{AddAccountView, ProviderView, ProvidersPayload};

use super::table::table;

const HEADER: [&str; 5] = ["ID", "NAME", "ADD ACCOUNT", "ACCOUNTS", "LOCAL USAGE"];

pub fn render_providers(payload: &ProvidersPayload) -> String {
    let rows: Vec<Vec<String>> = payload.providers.iter().map(row).collect();
    format!("{}\n", table(&HEADER, &rows))
}

fn row(provider: &ProviderView) -> Vec<String> {
    let methods: Vec<String> = provider.add_account.iter().map(method).collect();
    vec![
        provider.id.to_string(),
        provider.display_name.to_owned(),
        methods.join(" or "),
        if provider.multi_account {
            "several"
        } else {
            "one"
        }
        .to_owned(),
        if provider.local_usage { "yes" } else { "no" }.to_owned(),
    ]
}

fn method(method: &AddAccountView) -> String {
    match method {
        AddAccountView::CliLogin { program } => format!("sign in with {program}"),
        AddAccountView::ApiKey { label, .. } => format!("paste {}", label.to_lowercase()),
        AddAccountView::AutoDetect { .. } => "detected automatically".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderId;

    use super::*;

    #[test]
    fn every_provider_is_one_row_with_its_methods() {
        let keyed = ProviderView {
            id: ProviderId::from_static("keyed"),
            display_name: "Keyed",
            add_account: vec![
                AddAccountView::ApiKey {
                    label: "API key",
                    console_url: "https://keyed.example",
                    hint: "",
                },
                AddAccountView::AutoDetect { reason: "env" },
            ],
            multi_account: false,
            local_usage: false,
        };
        let mut payload = crate::providers::catalog().payload();
        payload.providers.push(keyed);
        let expected = "\
ID      NAME    ADD ACCOUNT                              ACCOUNTS  LOCAL USAGE
codex   Codex   sign in with codex                       several   yes
claude  Claude  sign in with claude                      several   yes
keyed   Keyed   paste api key or detected automatically  one       no
";
        assert_eq!(render_providers(&payload), expected);
    }
}

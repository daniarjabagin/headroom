use super::*;

const REGISTRY: &str = r#"{"version":1,"providers":[
    {"id":"codex","display_name":"Codex","add_account":[{"kind":"cli_login","program":"codex"}],"multi_account":true},
    {"id":"zai","display_name":"Z.ai","add_account":[{"kind":"api_key","label":"API key","console_url":"https://z.ai/keys","hint":"Starts with sk-"}]},
    {"id":"grok","display_name":"Grok","add_account":[{"kind":"cli_login"},{"kind":"api_key","console_url":"http://insecure"}]},
    {"id":"antigravity","display_name":" ","add_account":[{"kind":"auto_detect","reason":"Open Antigravity once"}]},
    {"id":"future","display_name":"Future","add_account":[{"kind":"telepathy"}]},
    {"id":"Bad Id","display_name":"Bad","add_account":[{"kind":"cli_login"}]},
    {"id":"codex","display_name":"Duplicate","add_account":[{"kind":"cli_login"}]},
    42
]}"#;

#[test]
fn parses_the_methods_of_each_provider() {
    let providers = parse_providers(REGISTRY).unwrap();
    let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["codex", "zai", "grok", "antigravity"]);
    assert_eq!(
        providers[0].methods,
        [AddMethod::CliLogin {
            program: Some("codex".into())
        }]
    );
    assert_eq!(
        providers[1].methods,
        [AddMethod::ApiKey {
            label: Some("API key".into()),
            console_url: Some("https://z.ai/keys".into()),
            hint: Some("Starts with sk-".into())
        }]
    );
    assert_eq!(
        providers[2].methods[1],
        AddMethod::ApiKey {
            label: None,
            console_url: None,
            hint: None
        }
    );
    assert_eq!(providers[3].display_name, "antigravity");
}

#[test]
fn keeps_only_https_links() {
    let json = r#"{"version":1,"providers":[
        {"id":"copilot","display_name":"Copilot","add_account":[{"kind":"cli_login"}],
         "links":{"status":"https://www.githubstatus.com","dashboard":"https://github.com/settings/copilot",
                  "usage":"http://github.com/settings/billing"}},
        {"id":"grok","display_name":"Grok","add_account":[{"kind":"cli_login"}],
         "links":{"status":null,"dashboard":"https://grok.com/ bad","usage":null}},
        {"id":"old","display_name":"Old","add_account":[{"kind":"cli_login"}],"links":7}
    ]}"#;
    let providers = parse_providers(json).unwrap();
    assert_eq!(
        providers[0].links,
        ProviderLinks {
            status: Some("https://www.githubstatus.com".into()),
            dashboard: Some("https://github.com/settings/copilot".into()),
            usage: None,
        }
    );
    assert_eq!(providers[1].links, ProviderLinks::default());
    assert_eq!(providers[2].links, ProviderLinks::default());
    let missing = parse_providers(REGISTRY).unwrap();
    assert_eq!(missing[0].links, ProviderLinks::default());
}

#[test]
fn rejects_unreadable_and_unknown_versions() {
    assert!(matches!(
        parse_providers("nope"),
        Err(RegistryError::Unreadable(_))
    ));
    let error = parse_providers(r#"{"version":2,"providers":[]}"#).unwrap_err();
    assert_eq!(error, RegistryError::Version(2));
    assert_eq!(
        error.message(Lang::En),
        "Headroom service lists providers in version 2, expected 1"
    );
}

#[test]
fn summaries_name_each_method() {
    let providers = parse_providers(REGISTRY).unwrap();
    assert_eq!(
        provider_summary(Lang::En, &providers[0]),
        "Sign in with codex"
    );
    assert_eq!(
        provider_summary(Lang::En, &providers[2]),
        "Sign in · API key"
    );
    assert_eq!(
        provider_summary(Lang::Ru, &providers[3]),
        "Находится автоматически"
    );
}

#[test]
fn builds_add_arguments_without_the_key() {
    let cli = AddMethod::CliLogin { program: None };
    assert_eq!(
        add_account_args("codex", &cli, "  "),
        ["accounts", "add", "codex", "--progress", "json"]
    );
    let key = AddMethod::ApiKey {
        label: None,
        console_url: None,
        hint: None,
    };
    assert_eq!(
        add_account_args("zai", &key, " Work "),
        [
            "accounts",
            "add",
            "zai",
            "--api-key-stdin",
            "--label=Work",
            "--progress",
            "json"
        ]
    );
}

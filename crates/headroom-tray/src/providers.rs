use std::collections::BTreeSet;

use serde::Deserialize;

const TERMINAL_METHODS: [&str; 2] = ["cli_login", "api_key"];

#[derive(Debug, Deserialize)]
struct Registry {
    providers: Vec<Provider>,
}

#[derive(Debug, Deserialize)]
struct Provider {
    id: String,
    add_account: Vec<AddAccount>,
}

#[derive(Debug, Deserialize)]
struct AddAccount {
    kind: String,
}

#[must_use]
pub fn terminal_sign_in(registry_json: &str) -> BTreeSet<String> {
    serde_json::from_str::<Registry>(registry_json)
        .map(|registry| {
            registry
                .providers
                .into_iter()
                .filter(|provider| {
                    provider
                        .add_account
                        .first()
                        .is_some_and(|method| TERMINAL_METHODS.contains(&method.kind.as_str()))
                })
                .map(|provider| provider.id)
                .collect()
        })
        .unwrap_or_default()
}

fn valid_provider_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[must_use]
pub fn sign_in_command(provider: &str, close_prompt: &str) -> Option<Vec<String>> {
    if !valid_provider_id(provider) {
        return None;
    }
    let login = format!(
        "headroom accounts add {provider}; status=$?; printf '\\n%s ' {}; read -r _; exit $status",
        shell_quote(close_prompt)
    );
    let quoted = shell_quote(&login);
    let launcher = format!(
        "if command -v xdg-terminal-exec >/dev/null 2>&1; then exec xdg-terminal-exec sh -c {quoted}; \
         else exec x-terminal-emulator -e sh -c {quoted}; fi"
    );
    Some(vec!["sh".into(), "-c".into(), launcher])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_providers_that_sign_in_through_a_terminal() {
        let json = r#"{"version":1,"providers":[
            {"id":"codex","display_name":"Codex","add_account":[{"kind":"cli_login","program":"codex"}],"multi_account":true,"local_usage":true},
            {"id":"zai","display_name":"Z.ai","add_account":[{"kind":"api_key","label":"API key","console_url":"https://z.ai","hint":null}],"multi_account":true,"local_usage":false},
            {"id":"antigravity","display_name":"Antigravity","add_account":[{"kind":"auto_detect","reason":"x"}],"multi_account":false,"local_usage":false}
        ]}"#;
        let ids: Vec<String> = terminal_sign_in(json).into_iter().collect();
        assert_eq!(ids, ["codex", "zai"]);
        assert!(terminal_sign_in("nope").is_empty());
    }

    #[test]
    fn builds_a_quoted_terminal_command() {
        let command = sign_in_command("codex", "Press Enter to close").unwrap();
        assert_eq!(command[..2], ["sh", "-c"]);
        assert!(command[2].contains("xdg-terminal-exec sh -c 'headroom accounts add codex;"));
        assert!(command[2].contains(r"'\''Press Enter to close'\''"));
        assert!(sign_in_command("codex; rm -rf ~", "x").is_none());
    }
}

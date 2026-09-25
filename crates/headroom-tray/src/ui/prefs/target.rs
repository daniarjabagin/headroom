use crate::i18n::{Lang, fill};
use crate::preferences::registry::{AddMethod, ProviderInfo, add_account_args};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Add,
    Login { account_id: String },
}

impl Target {
    #[must_use]
    pub fn args(&self, provider_id: &str, method: &AddMethod, label: &str) -> Vec<String> {
        match self {
            Target::Add => add_account_args(provider_id, method, label),
            Target::Login { account_id } => login_args(account_id, method),
        }
    }

    #[must_use]
    pub fn is_login(&self) -> bool {
        matches!(self, Target::Login { .. })
    }

    #[must_use]
    pub fn title(&self, lang: Lang, provider_name: &str) -> String {
        let template = match self {
            Target::Add => lang.tr("Add {provider} Account"),
            Target::Login { .. } => lang.tr("Sign In to {provider} Again"),
        };
        fill(template, &[("provider", provider_name)])
    }

    #[must_use]
    pub fn done_text(&self, lang: Lang) -> &'static str {
        match self {
            Target::Add => lang.tr("Account added. It shows up in the tray in a moment."),
            Target::Login { .. } => lang.tr("Signed in again. The account updates in a moment."),
        }
    }
}

fn login_args(account_id: &str, method: &AddMethod) -> Vec<String> {
    let mut args: Vec<String> = vec!["accounts".into(), "login".into(), account_id.into()];
    if matches!(method, AddMethod::ApiKey { .. }) {
        args.push("--api-key-stdin".into());
    }
    args.extend(["--progress".into(), "json".into()]);
    args
}

#[must_use]
pub fn login_method(provider: &ProviderInfo) -> Option<&AddMethod> {
    provider
        .methods
        .iter()
        .find(|method| matches!(method, AddMethod::CliLogin { .. }))
        .or_else(|| {
            provider
                .methods
                .iter()
                .find(|method| matches!(method, AddMethod::ApiKey { .. }))
        })
}

#[must_use]
pub fn provider_of_account(account_id: &str) -> &str {
    account_id
        .split_once(':')
        .map_or(account_id, |(provider, _)| provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::registry::ProviderLinks;

    fn cli() -> AddMethod {
        AddMethod::CliLogin {
            program: Some("claude".into()),
        }
    }

    fn key() -> AddMethod {
        AddMethod::ApiKey {
            label: None,
            console_url: None,
            hint: None,
        }
    }

    fn provider(methods: Vec<AddMethod>) -> ProviderInfo {
        ProviderInfo {
            id: "claude".into(),
            display_name: "Claude".into(),
            methods,
            links: ProviderLinks::default(),
        }
    }

    #[test]
    fn signing_in_again_runs_accounts_login() {
        let login = Target::Login {
            account_id: "claude:abc".into(),
        };
        assert_eq!(
            login.args("claude", &cli(), "ignored"),
            ["accounts", "login", "claude:abc", "--progress", "json"]
        );
        assert_eq!(
            login.args("claude", &key(), ""),
            [
                "accounts",
                "login",
                "claude:abc",
                "--api-key-stdin",
                "--progress",
                "json"
            ]
        );
        assert!(login.is_login());
        assert_eq!(
            Target::Add.args("claude", &cli(), "Work"),
            [
                "accounts",
                "add",
                "claude",
                "--label=Work",
                "--progress",
                "json"
            ]
        );
    }

    #[test]
    fn prefers_the_cli_sign_in_for_a_new_login() {
        let auto = AddMethod::AutoDetect { reason: None };
        assert_eq!(login_method(&provider(vec![key(), cli()])), Some(&cli()));
        assert_eq!(
            login_method(&provider(vec![auto.clone(), key()])),
            Some(&key())
        );
        assert_eq!(login_method(&provider(vec![auto])), None);
    }

    #[test]
    fn titles_name_the_flow() {
        let login = Target::Login {
            account_id: "x".into(),
        };
        assert_eq!(Target::Add.title(Lang::En, "Codex"), "Add Codex Account");
        assert_eq!(login.title(Lang::En, "Codex"), "Sign In to Codex Again");
        assert_eq!(login.title(Lang::Ru, "Codex"), "Повторный вход в Codex");
        assert_eq!(
            login.done_text(Lang::Ru),
            "Вход выполнен. Аккаунт скоро обновится."
        );
    }

    #[test]
    fn account_ids_start_with_the_provider() {
        assert_eq!(provider_of_account("codex:1a2b"), "codex");
        assert_eq!(provider_of_account("plain"), "plain");
    }
}

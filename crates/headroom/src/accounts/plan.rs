use anyhow::{Result, bail};
use headroom_core::descriptor::{AddAccountMethod, CliLogin, ProviderDescriptor};

use super::login::LoginSpec;

#[derive(Debug, Clone, Copy)]
pub enum AddPlan {
    Login(LoginSpec),
    ApiKey,
}

/// The API key method when a key is on stdin, otherwise the provider's first CLI login.
pub fn plan(descriptor: &'static ProviderDescriptor, api_key_stdin: bool) -> Result<AddPlan> {
    let name = descriptor.display_name;
    if api_key_stdin {
        if descriptor.accepts_api_key() {
            return Ok(AddPlan::ApiKey);
        }
        bail!("{name} accounts cannot be added with an API key");
    }
    if let Some(login) = cli_login(descriptor) {
        return Ok(AddPlan::Login(LoginSpec::new(descriptor, login)));
    }
    match descriptor.default_method() {
        Some(AddAccountMethod::ApiKey(prompt)) => bail!(
            "{name} accounts are added with an {}: pass --api-key-stdin and write it to stdin",
            prompt.label
        ),
        Some(AddAccountMethod::AutoDetect { reason }) => {
            bail!("{name} accounts are detected automatically: {reason}")
        }
        None | Some(AddAccountMethod::CliLogin(_)) => bail!("{name} accounts cannot be added"),
    }
}

fn cli_login(descriptor: &'static ProviderDescriptor) -> Option<&'static CliLogin> {
    descriptor
        .add_account
        .iter()
        .find_map(|method| match method {
            AddAccountMethod::CliLogin(login) => Some(login),
            AddAccountMethod::ApiKey(_) | AddAccountMethod::AutoDetect { .. } => None,
        })
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderId;
    use headroom_core::descriptor::{ApiKeyPrompt, HomeVar};

    use super::*;

    const KEY: AddAccountMethod = AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "API key",
        console_url: "https://keyed.example/keys",
        hint: "",
    });

    static KEYED: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("keyed"),
        display_name: "Keyed",
        add_account: &[KEY],
        multi_account: true,
        local_usage: false,
    };

    static LOGIN_THEN_KEY: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("both"),
        display_name: "Both",
        add_account: &[
            AddAccountMethod::CliLogin(CliLogin {
                program: "both",
                args: &["login"],
                home_var: HomeVar::XdgBase {
                    var: "XDG_DATA_HOME",
                    subdir: "both",
                },
                credentials_file: "auth.json",
                needs_pty: false,
            }),
            KEY,
        ],
        multi_account: true,
        local_usage: false,
    };

    static KEY_THEN_LOGIN: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("keyfirst"),
        display_name: "Key first",
        add_account: &[
            KEY,
            AddAccountMethod::CliLogin(CliLogin {
                program: "keyfirst",
                args: &["login"],
                home_var: HomeVar::Direct("KEYFIRST_HOME"),
                credentials_file: "auth.json",
                needs_pty: false,
            }),
        ],
        multi_account: true,
        local_usage: false,
    };

    static DETECTED: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("found"),
        display_name: "Found",
        add_account: &[AddAccountMethod::AutoDetect {
            reason: "one IDE login per machine",
        }],
        multi_account: false,
        local_usage: false,
    };

    fn message(result: Result<AddPlan>) -> String {
        result.unwrap_err().to_string()
    }

    #[test]
    fn the_default_method_decides_without_a_key() {
        let codex = headroom_providers::registry::descriptor("codex").unwrap();
        match plan(codex, false).unwrap() {
            AddPlan::Login(spec) => assert_eq!(spec.login.program, "codex"),
            AddPlan::ApiKey => panic!("codex signs in with its CLI"),
        }
        assert!(matches!(
            plan(&LOGIN_THEN_KEY, false),
            Ok(AddPlan::Login(_))
        ));
        match plan(&KEY_THEN_LOGIN, false).unwrap() {
            AddPlan::Login(spec) => assert_eq!(spec.login.program, "keyfirst"),
            AddPlan::ApiKey => panic!("without a key the CLI login is used"),
        }
        assert!(matches!(plan(&KEY_THEN_LOGIN, true), Ok(AddPlan::ApiKey)));
        assert_eq!(
            message(plan(&KEYED, false)),
            "Keyed accounts are added with an API key: pass --api-key-stdin and write it to stdin"
        );
        assert_eq!(
            message(plan(&DETECTED, false)),
            "Found accounts are detected automatically: one IDE login per machine"
        );
    }

    #[test]
    fn a_key_on_stdin_picks_the_api_key_method() {
        assert!(matches!(plan(&KEYED, true), Ok(AddPlan::ApiKey)));
        assert!(matches!(plan(&LOGIN_THEN_KEY, true), Ok(AddPlan::ApiKey)));
        let codex = headroom_providers::registry::descriptor("codex").unwrap();
        assert_eq!(
            message(plan(codex, true)),
            "Codex accounts cannot be added with an API key"
        );
    }
}

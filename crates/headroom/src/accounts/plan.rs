use anyhow::{Result, bail};
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor};

use super::login::LoginSpec;

#[derive(Debug, Clone, Copy)]
pub enum AddPlan {
    Login(LoginSpec),
    ApiKey,
}

/// The provider's default way to add an account, or its API key method when a key is on stdin.
pub fn plan(descriptor: &'static ProviderDescriptor, api_key_stdin: bool) -> Result<AddPlan> {
    let name = descriptor.display_name;
    if api_key_stdin {
        if descriptor.accepts_api_key() {
            return Ok(AddPlan::ApiKey);
        }
        bail!("{name} accounts cannot be added with an API key");
    }
    match descriptor.default_method() {
        Some(AddAccountMethod::CliLogin(login)) => {
            Ok(AddPlan::Login(LoginSpec::new(descriptor, login)))
        }
        Some(AddAccountMethod::ApiKey(prompt)) => bail!(
            "{name} accounts are added with an {}: pass --api-key-stdin and write it to stdin",
            prompt.label
        ),
        Some(AddAccountMethod::AutoDetect { reason }) => {
            bail!("{name} accounts are detected automatically: {reason}")
        }
        None => bail!("{name} accounts cannot be added"),
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderId;
    use headroom_core::descriptor::{ApiKeyPrompt, CliLogin, HomeVar};

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

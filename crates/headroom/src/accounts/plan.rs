use anyhow::{Result, bail};
use headroom_core::descriptor::{AddAccountMethod, ApiKeyPrompt, CliLogin, ProviderDescriptor};

use super::login::LoginSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    Stdin,
    Terminal,
    Unavailable,
}

#[derive(Debug, Clone, Copy)]
pub enum AddPlan {
    Login(LoginSpec),
    KeyFromStdin,
    KeyFromPrompt(&'static ApiKeyPrompt),
}

pub fn choose_add_plan(descriptor: &'static ProviderDescriptor, keys: KeyInput) -> Result<AddPlan> {
    let name = descriptor.display_name;
    if keys == KeyInput::Stdin {
        if descriptor.accepts_api_key() {
            return Ok(AddPlan::KeyFromStdin);
        }
        bail!("{name} accounts cannot be added with an API key");
    }
    if let (KeyInput::Terminal, Some(AddAccountMethod::ApiKey(prompt))) =
        (keys, descriptor.default_method())
    {
        return Ok(AddPlan::KeyFromPrompt(prompt));
    }
    if let Some(login) = cli_login(descriptor) {
        return Ok(AddPlan::Login(LoginSpec::new(descriptor, login)));
    }
    match descriptor.default_method() {
        Some(AddAccountMethod::ApiKey(prompt)) => bail!(
            "{name} accounts need a key ({}): pass --api-key-stdin and write it to stdin",
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
                scrub_env: &[],
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
                scrub_env: &[],
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
        for keys in [KeyInput::Terminal, KeyInput::Unavailable] {
            match choose_add_plan(codex, keys).unwrap() {
                AddPlan::Login(spec) => assert_eq!(spec.login.program, "codex"),
                other => panic!("codex signs in with its CLI: {other:?}"),
            }
            assert!(matches!(
                choose_add_plan(&LOGIN_THEN_KEY, keys),
                Ok(AddPlan::Login(_))
            ));
        }
        match choose_add_plan(&KEY_THEN_LOGIN, KeyInput::Unavailable).unwrap() {
            AddPlan::Login(spec) => assert_eq!(spec.login.program, "keyfirst"),
            other => panic!("without a key the CLI login is used: {other:?}"),
        }
        assert!(matches!(
            choose_add_plan(&KEY_THEN_LOGIN, KeyInput::Terminal),
            Ok(AddPlan::KeyFromPrompt(_))
        ));
        assert!(matches!(
            choose_add_plan(&KEY_THEN_LOGIN, KeyInput::Stdin),
            Ok(AddPlan::KeyFromStdin)
        ));
        assert_eq!(
            message(choose_add_plan(&KEYED, KeyInput::Unavailable)),
            "Keyed accounts need a key (API key): pass --api-key-stdin and write it to stdin"
        );
        assert_eq!(
            message(choose_add_plan(&DETECTED, KeyInput::Terminal)),
            "Found accounts are detected automatically: one IDE login per machine"
        );
    }

    #[test]
    fn a_key_on_stdin_picks_the_api_key_method() {
        assert!(matches!(
            choose_add_plan(&KEYED, KeyInput::Stdin),
            Ok(AddPlan::KeyFromStdin)
        ));
        assert!(matches!(
            choose_add_plan(&LOGIN_THEN_KEY, KeyInput::Stdin),
            Ok(AddPlan::KeyFromStdin)
        ));
        let codex = headroom_providers::registry::descriptor("codex").unwrap();
        assert_eq!(
            message(choose_add_plan(codex, KeyInput::Stdin)),
            "Codex accounts cannot be added with an API key"
        );
    }

    #[test]
    fn a_terminal_is_prompted_for_the_default_api_key() {
        match choose_add_plan(&KEYED, KeyInput::Terminal).unwrap() {
            AddPlan::KeyFromPrompt(prompt) => {
                assert_eq!(prompt.console_url, "https://keyed.example/keys");
            }
            other => panic!("expected a prompt: {other:?}"),
        }
        let opencode = headroom_providers::registry::descriptor("opencode").unwrap();
        assert!(matches!(
            choose_add_plan(opencode, KeyInput::Terminal),
            Ok(AddPlan::KeyFromPrompt(_))
        ));
        assert_eq!(
            message(choose_add_plan(opencode, KeyInput::Unavailable)),
            "OpenCode accounts need a key (API key): pass --api-key-stdin and write it to stdin"
        );
        let minimax = headroom_providers::registry::descriptor("minimax").unwrap();
        assert_eq!(
            message(choose_add_plan(minimax, KeyInput::Unavailable)),
            "MiniMax accounts need a key (MiniMax Token Plan key): pass --api-key-stdin and write it to stdin"
        );
    }
}

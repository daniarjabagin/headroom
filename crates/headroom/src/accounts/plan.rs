use anyhow::{Result, bail};
use headroom_core::descriptor::{AddAccountMethod, ApiKeyPrompt, ProviderDescriptor};

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
    if let Some(login) = descriptor.cli_login() {
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

pub fn choose_login_plan(
    descriptor: &'static ProviderDescriptor,
    holds_key: bool,
    keys: KeyInput,
) -> Result<AddPlan> {
    let name = descriptor.display_name;
    if !holds_key {
        let Some(login) = descriptor.cli_login() else {
            bail!("{name} accounts cannot be signed in again from Headroom");
        };
        if keys == KeyInput::Stdin {
            bail!(
                "this {name} account signs in with `{}`, not an API key",
                login.command_line()
            );
        }
        return Ok(AddPlan::Login(LoginSpec::new(descriptor, login)));
    }
    let Some(prompt) = api_key_prompt(descriptor) else {
        bail!("{name} accounts cannot be signed in with an API key");
    };
    match keys {
        KeyInput::Stdin => Ok(AddPlan::KeyFromStdin),
        KeyInput::Terminal => Ok(AddPlan::KeyFromPrompt(prompt)),
        KeyInput::Unavailable => bail!(
            "{name} accounts need a key ({}): pass --api-key-stdin and write it to stdin",
            prompt.label
        ),
    }
}

fn api_key_prompt(descriptor: &'static ProviderDescriptor) -> Option<&'static ApiKeyPrompt> {
    descriptor
        .add_account
        .iter()
        .find_map(|method| match method {
            AddAccountMethod::ApiKey(prompt) => Some(prompt),
            AddAccountMethod::CliLogin(_) | AddAccountMethod::AutoDetect { .. } => None,
        })
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
        links: headroom_core::descriptor::ProviderLinks::NONE,
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
                default_dir: ".tool",
                needs_pty: false,
                scrub_env: &[],
            }),
            KEY,
        ],
        multi_account: true,
        local_usage: false,
        links: headroom_core::descriptor::ProviderLinks::NONE,
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
                default_dir: ".tool",
                needs_pty: false,
                scrub_env: &[],
            }),
        ],
        multi_account: true,
        local_usage: false,
        links: headroom_core::descriptor::ProviderLinks::NONE,
    };

    static DETECTED: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("found"),
        display_name: "Found",
        add_account: &[AddAccountMethod::AutoDetect {
            reason: "one IDE login per machine",
        }],
        multi_account: false,
        local_usage: false,
        links: headroom_core::descriptor::ProviderLinks::NONE,
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

    #[test]
    fn signing_in_again_follows_what_the_home_holds() {
        let codex = headroom_providers::registry::descriptor("codex").unwrap();
        for keys in [KeyInput::Terminal, KeyInput::Unavailable] {
            match choose_login_plan(codex, false, keys).unwrap() {
                AddPlan::Login(spec) => assert_eq!(spec.login.program, "codex"),
                other => panic!("a CLI home signs in with its CLI: {other:?}"),
            }
            assert!(matches!(
                choose_login_plan(&KEY_THEN_LOGIN, false, keys),
                Ok(AddPlan::Login(_))
            ));
        }
        assert_eq!(
            message(choose_login_plan(codex, false, KeyInput::Stdin)),
            "this Codex account signs in with `codex login`, not an API key"
        );
        assert_eq!(
            message(choose_login_plan(&KEYED, false, KeyInput::Terminal)),
            "Keyed accounts cannot be signed in again from Headroom"
        );
        assert_eq!(
            message(choose_login_plan(&DETECTED, false, KeyInput::Terminal)),
            "Found accounts cannot be signed in again from Headroom"
        );
    }

    #[test]
    fn a_home_with_a_key_asks_for_a_new_key() {
        assert!(matches!(
            choose_login_plan(&LOGIN_THEN_KEY, true, KeyInput::Stdin),
            Ok(AddPlan::KeyFromStdin)
        ));
        match choose_login_plan(&LOGIN_THEN_KEY, true, KeyInput::Terminal).unwrap() {
            AddPlan::KeyFromPrompt(prompt) => {
                assert_eq!(prompt.console_url, "https://keyed.example/keys");
            }
            other => panic!("expected a prompt: {other:?}"),
        }
        assert_eq!(
            message(choose_login_plan(&KEYED, true, KeyInput::Unavailable)),
            "Keyed accounts need a key (API key): pass --api-key-stdin and write it to stdin"
        );
        let codex = headroom_providers::registry::descriptor("codex").unwrap();
        assert_eq!(
            message(choose_login_plan(codex, true, KeyInput::Stdin)),
            "Codex accounts cannot be signed in with an API key"
        );
    }
}

use super::*;

const LOGIN: CliLogin = CliLogin {
    program: "tool",
    args: &["login"],
    home_var: HomeVar::Direct("TOOL_HOME"),
    credentials_file: "auth.json",
    needs_pty: false,
    scrub_env: &[],
};

const KEY: ApiKeyPrompt = ApiKeyPrompt {
    label: "API key",
    console_url: "https://example.com/keys",
    hint: "Starts with sk-",
};

fn descriptor(add_account: &'static [AddAccountMethod]) -> ProviderDescriptor {
    ProviderDescriptor {
        id: ProviderId::from_static("tool"),
        display_name: "Tool",
        add_account,
        multi_account: true,
        local_usage: false,
    }
}

fn problem(add_account: &'static [AddAccountMethod]) -> Option<&'static str> {
    descriptor(add_account).validate().err().map(|e| e.problem)
}

#[test]
fn well_formed_descriptors_pass() {
    static METHODS: [AddAccountMethod; 3] = [
        AddAccountMethod::CliLogin(LOGIN),
        AddAccountMethod::ApiKey(KEY),
        AddAccountMethod::AutoDetect {
            reason: "one login",
        },
    ];
    let valid = descriptor(&METHODS);
    assert_eq!(valid.validate(), Ok(()));
    assert_eq!(valid.default_method(), METHODS.first());
    assert!(valid.accepts_api_key());
    assert!(!descriptor(&[AddAccountMethod::CliLogin(LOGIN)]).accepts_api_key());
}

#[test]
fn the_cli_login_is_the_first_login_method_as_typed() {
    static METHODS: [AddAccountMethod; 2] = [
        AddAccountMethod::ApiKey(KEY),
        AddAccountMethod::CliLogin(LOGIN),
    ];
    let login = descriptor(&METHODS).cli_login().unwrap();
    assert_eq!(login.command_line(), "tool login");
    assert_eq!(
        descriptor(&[AddAccountMethod::ApiKey(KEY)]).cli_login(),
        None
    );
}

#[test]
fn identity_problems_are_reported() {
    let mut bad = descriptor(&[]);
    assert_eq!(
        bad.validate().unwrap_err().problem,
        "no way to add an account"
    );
    bad.display_name = " ";
    assert_eq!(bad.validate().unwrap_err().problem, "display name is empty");
    bad.id = ProviderId::from_static("Tool");
    let error = bad.validate().unwrap_err();
    assert!(error.to_string().starts_with("provider Tool: id must be"));
}

#[test]
fn login_specs_must_stay_inside_the_home() {
    static PATH_PROGRAM: [AddAccountMethod; 1] = [AddAccountMethod::CliLogin(CliLogin {
        program: "/usr/bin/tool",
        ..LOGIN
    })];
    static ESCAPING_FILE: [AddAccountMethod; 1] = [AddAccountMethod::CliLogin(CliLogin {
        credentials_file: "../auth.json",
        ..LOGIN
    })];
    static LOWER_VAR: [AddAccountMethod; 1] = [AddAccountMethod::CliLogin(CliLogin {
        home_var: HomeVar::Direct("tool_home"),
        ..LOGIN
    })];
    static ABSOLUTE_SUBDIR: [AddAccountMethod; 1] = [AddAccountMethod::CliLogin(CliLogin {
        home_var: HomeVar::XdgBase {
            var: "XDG_DATA_HOME",
            subdir: "/tool",
        },
        ..LOGIN
    })];
    static SCRUBBED_HOME: [AddAccountMethod; 1] = [AddAccountMethod::CliLogin(CliLogin {
        scrub_env: &["TOOL_HOME"],
        ..LOGIN
    })];
    static LOWER_SCRUB: [AddAccountMethod; 1] = [AddAccountMethod::CliLogin(CliLogin {
        scrub_env: &["tool_token"],
        ..LOGIN
    })];
    assert!(problem(&SCRUBBED_HOME).unwrap().contains("scrubbed"));
    assert!(problem(&LOWER_SCRUB).unwrap().contains("scrubbed"));
    assert!(problem(&PATH_PROGRAM).unwrap().contains("bare command"));
    assert!(
        problem(&ESCAPING_FILE)
            .unwrap()
            .contains("credentials file")
    );
    assert!(
        problem(&LOWER_VAR)
            .unwrap()
            .contains("environment variable")
    );
    assert!(
        problem(&ABSOLUTE_SUBDIR)
            .unwrap()
            .contains("XDG subdirectory")
    );
}

#[test]
fn key_and_auto_detect_specs_need_their_texts() {
    static PLAIN_URL: [AddAccountMethod; 1] = [AddAccountMethod::ApiKey(ApiKeyPrompt {
        console_url: "http://example.com",
        ..KEY
    })];
    static NO_LABEL: [AddAccountMethod; 1] =
        [AddAccountMethod::ApiKey(ApiKeyPrompt { label: "", ..KEY })];
    static NO_REASON: [AddAccountMethod; 1] = [AddAccountMethod::AutoDetect { reason: " " }];
    assert!(problem(&PLAIN_URL).unwrap().contains("https"));
    assert!(problem(&NO_LABEL).unwrap().contains("label"));
    assert!(problem(&NO_REASON).unwrap().contains("reason"));
}

#[test]
fn credentials_resolve_below_the_home_var_target() {
    let home = Path::new("/data/accounts/tool/1");
    assert_eq!(LOGIN.credentials_path(home), home.join("auth.json"));
    let xdg = CliLogin {
        home_var: HomeVar::XdgBase {
            var: "XDG_DATA_HOME",
            subdir: "tool",
        },
        ..LOGIN
    };
    assert_eq!(xdg.credentials_path(home), home.join("tool/auth.json"));
    assert_eq!(xdg.home_var.var(), "XDG_DATA_HOME");
}

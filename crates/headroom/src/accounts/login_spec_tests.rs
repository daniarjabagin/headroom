use headroom_core::descriptor::{AddAccountMethod, HomeVar};

use super::test_support::{FakeBin, homes, spec};
use super::*;

static XDG_TOOL: ProviderDescriptor = ProviderDescriptor {
    id: ProviderId::from_static("xdgtool"),
    display_name: "XDG Tool",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "xdgtool",
        args: &["auth", "login"],
        home_var: HomeVar::XdgBase {
            var: "XDG_DATA_HOME",
            subdir: "xdgtool",
        },
        credentials_file: "auth.json",
        needs_pty: false,
    })],
    multi_account: true,
    local_usage: false,
};

static PTY_LOGIN: CliLogin = CliLogin {
    program: "ptytool",
    args: &["login"],
    home_var: HomeVar::Direct("PTYTOOL_DIR"),
    credentials_file: "auth.json",
    needs_pty: true,
};

fn xdg_spec() -> LoginSpec {
    match &XDG_TOOL.add_account[0] {
        AddAccountMethod::CliLogin(login) => LoginSpec::new(&XDG_TOOL, login),
        other => panic!("{other:?}"),
    }
}

#[test]
fn login_commands_point_the_cli_at_the_new_home() {
    let home = Path::new("/data/headroom/accounts/codex/1");
    assert_eq!(
        spec("codex").display(home),
        "CODEX_HOME=/data/headroom/accounts/codex/1 codex login"
    );
    assert_eq!(
        spec("claude").display(home),
        "CLAUDE_CONFIG_DIR=/data/headroom/accounts/codex/1 claude auth login --claudeai"
    );
}

#[test]
fn xdg_base_logins_point_the_base_at_the_home_and_wait_for_the_subdir() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let spec = xdg_spec();
    assert_eq!(
        spec.display(Path::new("/h")),
        "XDG_DATA_HOME=/h xdgtool auth login"
    );
    bin.install("xdgtool", r#"echo '{}' > "$XDG_DATA_HOME/auth.json""#);
    let error = sign_in(
        root.path(),
        spec,
        &bin.launcher(),
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("wrote no auth.json"), "{error}");
    bin.install(
        "xdgtool",
        r#"PATH=/usr/bin:/bin; mkdir -p "$XDG_DATA_HOME/xdgtool" && echo '{}' > "$XDG_DATA_HOME/xdgtool/auth.json""#,
    );
    let home = sign_in(
        root.path(),
        spec,
        &bin.launcher(),
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("xdgtool"));
    assert!(home.join("xdgtool/auth.json").is_file());
}

#[test]
fn logins_that_need_a_terminal_are_refused_before_creating_a_home() {
    let root = tempfile::tempdir().unwrap();
    let spec = LoginSpec {
        provider: &XDG_TOOL.id,
        login: &PTY_LOGIN,
    };
    let error = sign_in(
        root.path(),
        spec,
        &Launcher::default(),
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "`ptytool login` needs a terminal to sign in, which Headroom cannot provide yet"
    );
    assert!(homes(root.path(), "xdgtool").is_empty());
}

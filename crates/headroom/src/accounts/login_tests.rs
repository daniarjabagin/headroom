use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;

use super::*;

struct FakeBin {
    dir: tempfile::TempDir,
}

impl FakeBin {
    fn new() -> FakeBin {
        FakeBin {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn install(&self, name: &str, body: &str) {
        let path = self.dir.path().join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o755)).unwrap();
    }

    fn launcher(&self) -> Launcher {
        Launcher {
            search_path: Some(self.dir.path().as_os_str().to_owned()),
        }
    }
}

fn homes(root: &Path, provider: &str) -> Vec<PathBuf> {
    match fs::read_dir(root.join(provider)) {
        Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn login_commands_point_the_cli_at_the_new_home() {
    let home = Path::new("/data/headroom/accounts/codex/1");
    assert_eq!(
        login_spec(ProviderKind::Codex).display(home),
        "CODEX_HOME=/data/headroom/accounts/codex/1 codex login"
    );
    assert_eq!(
        login_spec(ProviderKind::Claude).display(home),
        "CLAUDE_CONFIG_DIR=/data/headroom/accounts/codex/1 claude auth login --claudeai"
    );
}

#[test]
fn sign_in_runs_the_cli_and_cleans_up_failures() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let launcher = bin.launcher();

    bin.install(
        "codex",
        r#"printf '%s\n' "$@" > "$CODEX_HOME/args"; echo '{}' > "$CODEX_HOME/auth.json""#,
    );
    let home = sign_in(root.path(), ProviderKind::Codex, &launcher).unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("codex"));
    assert_eq!(fs::read_to_string(home.join("args")).unwrap(), "login\n");
    assert_eq!(
        fs::metadata(&home).unwrap().permissions().mode() & 0o777,
        0o700
    );

    bin.install(
        "claude",
        r#"printf '%s\n' "$@" > "$CLAUDE_CONFIG_DIR/args"; echo '{}' > "$CLAUDE_CONFIG_DIR/.credentials.json""#,
    );
    let home = sign_in(root.path(), ProviderKind::Claude, &launcher).unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("claude"));
    assert_eq!(
        fs::read_to_string(home.join("args")).unwrap(),
        "auth\nlogin\n--claudeai\n"
    );

    bin.install(
        "claude",
        r#"echo partial > "$CLAUDE_CONFIG_DIR/log"; exit 3"#,
    );
    let error = sign_in(root.path(), ProviderKind::Claude, &launcher).unwrap_err();
    assert!(
        error.to_string().contains("did not finish successfully"),
        "{error}"
    );
    assert_eq!(homes(root.path(), "claude").len(), 1);

    bin.install("codex", "exit 0");
    let error = sign_in(root.path(), ProviderKind::Codex, &launcher).unwrap_err();
    assert!(error.to_string().contains("wrote no auth.json"), "{error}");
    assert_eq!(homes(root.path(), "codex").len(), 1);

    let empty = FakeBin::new();
    let error = sign_in(root.path(), ProviderKind::Codex, &empty.launcher()).unwrap_err();
    assert!(error.to_string().contains("is it installed"), "{error}");
    assert_eq!(homes(root.path(), "codex").len(), 1);
}

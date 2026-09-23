use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const FAKE_GH: &str = r#"#!/bin/sh
[ "$1 $2 $3 $4 $5" = "auth token --hostname github.com --user" ] || exit 2
[ -z "$GH_TOKEN$GITHUB_TOKEN" ] || exit 3
[ "$GH_PROMPT_DISABLED" = "1" ] || exit 4
token="$GH_CONFIG_DIR/fake-tokens/$6"
if [ ! -f "$token" ]; then
    echo "no oauth token found for github.com account $6" >&2
    exit 1
fi
cat "$token"
echo
"#;

pub(super) fn fake_gh_printing_stored_tokens(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let program = bin.join("gh");
    fs::write(&program, FAKE_GH).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

pub(super) fn sign_in(config_dir: &Path, hosts: &str, tokens: &[(&str, &str)]) {
    let token_dir = config_dir.join("fake-tokens");
    fs::create_dir_all(&token_dir).unwrap();
    fs::write(config_dir.join("hosts.yml"), hosts).unwrap();
    for (login, token) in tokens {
        fs::write(token_dir.join(login), token).unwrap();
    }
}

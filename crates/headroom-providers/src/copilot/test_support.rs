use std::fs;
use std::path::{Path, PathBuf};

use crate::test_support::install_script;

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
    install_script(&program, FAKE_GH).unwrap();
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

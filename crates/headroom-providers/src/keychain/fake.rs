use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::Security;

const SCRIPT: &str = r#"#!/bin/sh
dir="$(dirname "$0")"
printf '%s\n' "$*" >> "$dir/argv.log"
if [ -f "$dir/hang" ]; then exec sleep 30; fi
if [ -f "$dir/exit_code" ]; then exit "$(cat "$dir/exit_code")"; fi
hex_decode() {
    awk '{ h = "0123456789abcdef"; s = tolower($0);
           for (i = 1; i < length(s); i += 2)
               printf "%c", (index(h, substr(s, i, 1)) - 1) * 16 + index(h, substr(s, i + 1, 1)) - 1 }'
}
item_path() {
    printf '%s/items/%s/%s' "$dir" "$1" "${2:-_}"
}
parse() {
    command="$1"; shift
    service=""; account=""; hex=""
    while [ $# -gt 0 ]; do
        case "$1" in
            -s) service="$2"; shift 2 ;;
            -a) account="$2"; shift 2 ;;
            -X) hex="$2"; shift 2 ;;
            *) shift ;;
        esac
    done
}
if [ "$1" = "-i" ]; then
    line="$(cat)"
    printf '%s\n' "$line" >> "$dir/stdin.log"
    eval "set -- $line"
    parse "$@"
    [ "$command" = "add-generic-password" ] || exit 2
    mkdir -p "$dir/items/$service"
    printf '%s' "$hex" | hex_decode > "$(item_path "$service" "$account")"
    exit 0
fi
parse "$@"
path="$(item_path "$service" "$account")"
if [ -z "$account" ]; then
    path="$(ls -d "$dir/items/$service"/* 2>/dev/null | head -n 1)"
fi
[ -n "$path" ] && [ -f "$path" ] || exit 44
case "$command" in
    find-generic-password) cat "$path"; echo ;;
    delete-generic-password) rm "$path" ;;
    *) exit 2 ;;
esac
"#;

/// A stand-in for `/usr/bin/security` that keeps items as files in a temp dir.
pub(crate) struct FakeKeychain {
    dir: PathBuf,
}

impl FakeKeychain {
    pub(crate) fn new(root: &Path) -> FakeKeychain {
        let dir = root.join("fake-security");
        fs::create_dir_all(&dir).unwrap();
        let program = dir.join("security");
        fs::write(&program, SCRIPT).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        FakeKeychain { dir }
    }

    pub(crate) fn security(&self) -> Security {
        self.security_with_timeout(Duration::from_secs(5))
    }

    pub(crate) fn security_with_timeout(&self, timeout: Duration) -> Security {
        Security::new(self.dir.join("security"), timeout)
    }

    pub(crate) fn insert(&self, service: &str, account: Option<&str>, secret: &str) {
        let dir = self.dir.join("items").join(service);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(account.unwrap_or("_")), secret).unwrap();
    }

    pub(crate) fn item(&self, service: &str, account: Option<&str>) -> Option<String> {
        let path = self
            .dir
            .join("items")
            .join(service)
            .join(account.unwrap_or("_"));
        fs::read_to_string(path).ok()
    }

    pub(crate) fn fail_with(&self, code: i32) {
        fs::write(self.dir.join("exit_code"), code.to_string()).unwrap();
    }

    pub(crate) fn hang(&self) {
        fs::write(self.dir.join("hang"), "").unwrap();
    }

    pub(crate) fn argv_log(&self) -> String {
        fs::read_to_string(self.dir.join("argv.log")).unwrap_or_default()
    }

    pub(crate) fn stdin_log(&self) -> String {
        fs::read_to_string(self.dir.join("stdin.log")).unwrap_or_default()
    }
}

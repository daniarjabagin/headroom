use std::fs;
use std::io;
use std::path::Path;

use headroom_core::provider::ProviderError;

pub(super) const HOSTS_FILE: &str = "hosts.yml";
pub(super) const GITHUB_HOST: &str = "github.com";
const MAX_LOGIN_LEN: usize = 100;

/// Accounts the GitHub CLI knows for github.com, the active one first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct GhUsers(pub(super) Vec<String>);

pub(super) fn load_users(config_dir: &Path) -> Result<GhUsers, ProviderError> {
    let path = config_dir.join(HOSTS_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(parse_users(&text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(GhUsers::default()),
        Err(error) => Err(ProviderError::LocalData(format!(
            "cannot read {}: {}",
            path.display(),
            error.kind()
        ))),
    }
}

#[derive(Default)]
struct Scan {
    in_host: bool,
    users_indent: Option<usize>,
    user_indent: Option<usize>,
    active: Option<String>,
    users: Vec<String>,
}

pub(super) fn parse_users(text: &str) -> GhUsers {
    let mut scan = Scan::default();
    for line in text.lines() {
        let content = line.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        scan.line(indent, content);
    }
    scan.finish()
}

impl Scan {
    fn line(&mut self, indent: usize, content: &str) {
        let Some((key, value)) = content.split_once(':') else {
            return;
        };
        let key = unquote(key.trim());
        if indent == 0 {
            self.in_host = key == GITHUB_HOST;
            self.users_indent = None;
            return;
        }
        if !self.in_host {
            return;
        }
        if let Some(users_indent) = self.users_indent
            && indent > users_indent
        {
            self.user_entry(indent, key);
            return;
        }
        self.users_indent = None;
        match key {
            "users" => {
                self.users_indent = Some(indent);
                self.user_indent = None;
            }
            "user" => self.active = Some(unquote(value.trim()).to_owned()),
            _ => {}
        }
    }

    fn user_entry(&mut self, indent: usize, key: &str) {
        let level = *self.user_indent.get_or_insert(indent);
        if indent == level {
            self.users.push(key.to_owned());
        }
    }

    fn finish(self) -> GhUsers {
        let mut users: Vec<String> = Vec::new();
        for login in self.active.into_iter().chain(self.users) {
            if is_login(&login) && !users.contains(&login) {
                users.push(login);
            }
        }
        GhUsers(users)
    }
}

fn unquote(text: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|quote| {
            text.strip_prefix(*quote)
                .and_then(|rest| rest.strip_suffix(*quote))
        })
        .unwrap_or(text)
}

fn is_login(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_LOGIN_LEN
        && !text.starts_with('-')
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTI: &str = include_str!("fixtures/hosts_multi.yml");
    const LEGACY: &str = include_str!("fixtures/hosts_legacy.yml");

    fn users(text: &str) -> Vec<String> {
        parse_users(text).0
    }

    #[test]
    fn every_github_com_user_is_listed_with_the_active_one_first() {
        assert_eq!(users(MULTI), ["octo-work", "octocat"]);
    }

    #[test]
    fn a_single_account_file_from_older_gh_lists_its_user() {
        assert_eq!(users(LEGACY), ["octocat"]);
    }

    #[test]
    fn other_hosts_nested_keys_and_odd_names_are_ignored() {
        let text = "ghe.corp.example:\n    user: enterprise\n    users:\n        enterprise:\n\
                    \"github.com\":\n    users:\n        'mona':\n            oauth_token: x\n\
                    \x20       bad name:\n        -dash:\n    user: \"mona\"\n";
        assert_eq!(users(text), ["mona"]);
        assert_eq!(users(""), Vec::<String>::new());
        assert_eq!(
            users("github.com:\n    git_protocol: https\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_missing_file_has_no_users() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_users(dir.path()), Ok(GhUsers::default()));
        fs::write(dir.path().join(HOSTS_FILE), MULTI).unwrap();
        assert_eq!(load_users(dir.path()).unwrap().0.len(), 2);
    }
}

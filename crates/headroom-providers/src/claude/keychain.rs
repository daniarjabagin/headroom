use std::path::Path;

use headroom_core::account::CredentialOwner;
use icu_normalizer::ComposingNormalizerBorrowed;
use sha2::{Digest, Sha256};

use super::config::ClaudeConfig;
use super::identity::same_dir;

pub(super) const SERVICE: &str = "Claude Code-credentials";
const SCOPE_HEX_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    pub(super) service: String,
    pub(super) account: Option<String>,
}

pub(super) fn scoped_service(config_dir: &str) -> String {
    let normalized = ComposingNormalizerBorrowed::new_nfc().normalize(config_dir);
    let digest = hex::encode(Sha256::digest(normalized.as_bytes()));
    let scope = digest.get(..SCOPE_HEX_LEN).unwrap_or(&digest);
    format!("{SERVICE}-{scope}")
}

pub(super) fn candidates(
    config: &ClaudeConfig,
    dir: &Path,
    owner: CredentialOwner,
) -> Vec<Candidate> {
    let accounts: Vec<Option<String>> = config
        .user
        .iter()
        .cloned()
        .map(Some)
        .chain(std::iter::once(None))
        .collect();
    services(config, dir, owner)
        .into_iter()
        .flat_map(|service| {
            accounts.iter().map(move |account| Candidate {
                service: service.clone(),
                account: account.clone(),
            })
        })
        .collect()
}

fn services(config: &ClaudeConfig, dir: &Path, owner: CredentialOwner) -> Vec<String> {
    if owner == CredentialOwner::Headroom {
        return path_service(dir).into_iter().collect();
    }
    match config.config_dir.as_deref() {
        Some(override_dir) if same_dir(override_dir, dir) => path_service(override_dir)
            .into_iter()
            .chain(std::iter::once(SERVICE.to_owned()))
            .collect(),
        _ if same_dir(dir, &config.default_dir()) => vec![SERVICE.to_owned()],
        _ => path_service(dir).into_iter().collect(),
    }
}

fn path_service(dir: &Path) -> Option<String> {
    dir.to_str().map(scoped_service)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn config(config_dir: Option<&str>, user: Option<&str>) -> ClaudeConfig {
        ClaudeConfig {
            config_dir: config_dir.map(PathBuf::from),
            user: user.map(str::to_owned),
            ..ClaudeConfig::for_home(PathBuf::from("/Users/u"))
        }
    }

    fn sha8(text: &str) -> String {
        hex::encode(Sha256::digest(text.as_bytes()))[..8].to_owned()
    }

    #[test]
    fn scoped_service_hashes_the_nfc_config_dir() {
        let composed = "/Users/u/Caf\u{e9}";
        let decomposed = "/Users/u/Cafe\u{301}";
        assert_eq!(
            scoped_service(composed),
            format!("Claude Code-credentials-{}", sha8(composed))
        );
        assert_eq!(scoped_service(decomposed), scoped_service(composed));
        assert_ne!(sha8(decomposed), sha8(composed));
    }

    #[test]
    fn the_default_dir_uses_the_plain_service_with_user_then_legacy() {
        let config = config(None, Some("u"));
        let found = candidates(&config, Path::new("/Users/u/.claude"), CredentialOwner::Cli);
        assert_eq!(
            found,
            [
                Candidate {
                    service: SERVICE.to_owned(),
                    account: Some("u".to_owned()),
                },
                Candidate {
                    service: SERVICE.to_owned(),
                    account: None,
                },
            ]
        );
    }

    #[test]
    fn an_override_tries_the_scoped_service_before_the_plain_one() {
        let config = config(Some("/work/claude"), None);
        let found = candidates(&config, Path::new("/work/claude"), CredentialOwner::Cli);
        let services: Vec<&str> = found.iter().map(|c| c.service.as_str()).collect();
        let scoped = format!("Claude Code-credentials-{}", sha8("/work/claude"));
        assert_eq!(services, [scoped.as_str(), SERVICE]);
        assert!(found.iter().all(|c| c.account.is_none()));
    }

    #[test]
    fn headroom_homes_only_use_their_own_scoped_service() {
        let config = config(None, Some("u"));
        let home = "/Users/u/Library/Application Support/Headroom/accounts/claude/x";
        let found = candidates(&config, Path::new(home), CredentialOwner::Headroom);
        let scoped = format!("Claude Code-credentials-{}", sha8(home));
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|c| c.service == scoped));
    }

    #[test]
    fn scanned_dirs_use_their_path_scope() {
        let config = config(None, None);
        let found = candidates(
            &config,
            Path::new("/Users/u/.claude-work"),
            CredentialOwner::Cli,
        );
        let scoped = format!("Claude Code-credentials-{}", sha8("/Users/u/.claude-work"));
        assert_eq!(
            found,
            [Candidate {
                service: scoped,
                account: None,
            }]
        );
    }
}

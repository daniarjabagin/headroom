use std::path::PathBuf;

use headroom_core::account::AccountId;

use super::*;

fn record(provider: &str, home: &str, owner: CredentialOwner) -> AccountRef {
    AccountRef {
        id: AccountId(format!("{provider}:0123456789ab")),
        provider: ProviderId::parse(provider).unwrap(),
        home: PathBuf::from(home),
        owner,
    }
}

fn refusal(result: Result<&AccountRef>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn only_headroom_owned_accounts_sign_in_again() {
    let own = record("codex", "/data/codex/1", CredentialOwner::Headroom);
    let accounts = std::slice::from_ref(&own);
    for shown in [
        None,
        Some(CredentialOwner::Headroom),
        Some(CredentialOwner::Cli),
    ] {
        assert_eq!(
            owned_record(accounts, "codex:0123456789ab", shown).unwrap(),
            &own
        );
    }
    assert_eq!(
        refusal(owned_record(accounts, "codex:ffffffffffff", None)),
        "no signed-in account codex:ffffffffffff found"
    );
}

#[test]
fn cli_accounts_point_at_their_own_login() {
    let cli = record("codex", "/home/ada/.codex", CredentialOwner::Cli);
    assert_eq!(
        refusal(owned_record(
            std::slice::from_ref(&cli),
            "codex:0123456789ab",
            None
        )),
        "This account belongs to the Codex CLI — run `codex login` instead"
    );
    let claude = record("claude", "/home/ada/.claude", CredentialOwner::Cli);
    assert_eq!(
        refusal(owned_record(
            std::slice::from_ref(&claude),
            "claude:0123456789ab",
            Some(CredentialOwner::Cli)
        )),
        "This account belongs to the Claude CLI — run `claude auth login --claudeai` instead"
    );
    assert_eq!(
        cli_owned(&ProviderId::parse("cursor").unwrap()),
        "This account belongs to Cursor outside Headroom — sign in there again"
    );
}

#[test]
fn the_record_the_daemon_shows_decides_between_two_homes() {
    let cli = record("codex", "/home/ada/.codex", CredentialOwner::Cli);
    let own = record("codex", "/data/codex/1", CredentialOwner::Headroom);
    let both = [cli, own.clone()];
    let id = "codex:0123456789ab";
    assert_eq!(owned_record(&both, id, None).unwrap(), &own);
    assert_eq!(
        owned_record(&both, id, Some(CredentialOwner::Headroom)).unwrap(),
        &own
    );
    assert!(refusal(owned_record(&both, id, Some(CredentialOwner::Cli))).contains("`codex login`"));
}

#[test]
fn another_identity_in_the_home_is_announced() {
    let previous = record("codex", "/data/codex/1", CredentialOwner::Headroom);
    assert_eq!(identity_note(&previous, &previous), None);
    let renewed = AccountRef {
        id: AccountId("codex:fedcba987654".into()),
        ..previous.clone()
    };
    assert_eq!(
        identity_note(&previous, &renewed).unwrap(),
        "Signed in with another account: /data/codex/1 now holds codex:fedcba987654 instead of \
         codex:0123456789ab; Headroom shows it as a new account."
    );
}

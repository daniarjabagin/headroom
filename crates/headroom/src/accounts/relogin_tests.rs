use std::path::PathBuf;

use headroom_core::account::{AccountId, ProviderId};

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
fn the_account_to_sign_in_again_must_be_signed_in_somewhere() {
    let own = record("codex", "/data/codex/1", CredentialOwner::Headroom);
    let accounts = std::slice::from_ref(&own);
    for shown in [
        None,
        Some(CredentialOwner::Headroom),
        Some(CredentialOwner::Cli),
    ] {
        assert_eq!(
            shown_record(accounts, "codex:0123456789ab", shown).unwrap(),
            &own
        );
    }
    assert_eq!(
        refusal(shown_record(accounts, "codex:ffffffffffff", None)),
        "no signed-in account codex:ffffffffffff found"
    );
}

#[test]
fn the_record_the_daemon_shows_decides_between_two_homes() {
    let cli = record("codex", "/home/ada/.codex", CredentialOwner::Cli);
    let own = record("codex", "/data/codex/1", CredentialOwner::Headroom);
    let both = [cli.clone(), own.clone()];
    let id = "codex:0123456789ab";
    assert_eq!(shown_record(&both, id, None).unwrap(), &own);
    assert_eq!(
        shown_record(&both, id, Some(CredentialOwner::Headroom)).unwrap(),
        &own
    );
    assert_eq!(
        shown_record(&both, id, Some(CredentialOwner::Cli)).unwrap(),
        &cli
    );
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

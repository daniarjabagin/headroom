use std::fs;

use headroom_core::account::{AccountId, AccountRef, CredentialOwner};

use crate::accounts::progress::JsonLines;
use crate::accounts::progress_event;

use super::test_support::{FakeBin, FixedAccounts, homes, spec};
use super::*;

const RENEWING_CODEX_LOGIN: &str = r#"
echo "https://auth.openai.com/oauth/authorize?client_id=x"
echo '{"tokens":"new"}' > "$CODEX_HOME/auth.json"
"#;

struct SignedIn {
    root: tempfile::TempDir,
    home: PathBuf,
}

impl SignedIn {
    fn new() -> SignedIn {
        let root = tempfile::tempdir().unwrap();
        let home = create_home(root.path(), spec("codex").provider).unwrap();
        fs::write(home.join("auth.json"), r#"{"tokens":"old"}"#).unwrap();
        SignedIn { root, home }
    }

    fn renew_streamed(&self, bin: &FakeBin) -> (Result<()>, String) {
        let codex = spec("codex");
        let mut out = JsonLines::new(Vec::new());
        let mut events = |event: LoginEvent| out.emit(&progress_event(codex, event));
        let console = Console::Streamed {
            input: Box::new(std::io::empty()),
            events: &mut events,
        };
        let result = sign_in_at(
            &self.home,
            codex,
            &bin.launcher(),
            console,
            &Cancel::default(),
        );
        (result, String::from_utf8(out.into_inner()).unwrap())
    }

    async fn confirm(&self, before: &CredentialsStamp) -> Result<()> {
        confirm_renewal(
            &FixedAccounts(Vec::new()),
            &spec("codex"),
            &self.home,
            before,
        )
        .await
    }
}

#[tokio::test]
async fn signing_in_again_reuses_the_home_and_reports_it() {
    let signed_in = SignedIn::new();
    let bin = FakeBin::new();
    bin.install("codex", RENEWING_CODEX_LOGIN);
    let before = CredentialsStamp::read(&spec("codex"), &signed_in.home).unwrap();
    let (result, json) = signed_in.renew_streamed(&bin);
    result.unwrap();
    signed_in.confirm(&before).await.unwrap();
    assert_eq!(
        homes(signed_in.root.path(), "codex"),
        std::slice::from_ref(&signed_in.home)
    );
    assert_eq!(
        fs::read_to_string(signed_in.home.join("auth.json")).unwrap(),
        "{\"tokens\":\"new\"}\n"
    );
    let home = signed_in.home.display();
    let url = "https://auth.openai.com/oauth/authorize?client_id=x";
    let expected = [
        format!(r#"{{"event":"started","provider":"codex","home":"{home}"}}"#),
        format!(r#"{{"event":"output","line":"{url}"}}"#),
        format!(r#"{{"event":"url","url":"{url}"}}"#),
    ];
    assert_eq!(json, expected.join("\n") + "\n");
}

#[tokio::test]
async fn a_login_that_changes_nothing_is_not_a_renewal() {
    let signed_in = SignedIn::new();
    let bin = FakeBin::new();
    bin.install("codex", "exit 0");
    let before = CredentialsStamp::read(&spec("codex"), &signed_in.home).unwrap();
    signed_in.renew_streamed(&bin).0.unwrap();
    let error = signed_in.confirm(&before).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "`codex login` finished but did not renew auth.json"
    );
    assert!(signed_in.home.join("auth.json").is_file());
}

#[test]
fn a_failed_login_keeps_the_home_and_its_credentials() {
    let signed_in = SignedIn::new();
    let bin = FakeBin::new();
    bin.install("codex", r#"echo "denied" >&2; exit 3"#);
    let (result, json) = signed_in.renew_streamed(&bin);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("did not finish successfully")
    );
    assert!(json.contains(r#""line":"denied""#), "{json}");
    assert_eq!(
        fs::read_to_string(signed_in.home.join("auth.json")).unwrap(),
        r#"{"tokens":"old"}"#
    );
    assert_eq!(
        homes(signed_in.root.path(), "codex"),
        std::slice::from_ref(&signed_in.home)
    );
}

#[tokio::test]
async fn credentials_kept_elsewhere_are_confirmed_by_the_provider() {
    let root = tempfile::tempdir().unwrap();
    let claude = spec("claude");
    let home = create_home(root.path(), claude.provider).unwrap();
    let before = CredentialsStamp::read(&claude, &home).unwrap();
    let account = AccountRef {
        id: AccountId("claude:0123456789ab".into()),
        provider: claude.provider.clone(),
        home: home.clone(),
        owner: CredentialOwner::Headroom,
    };
    confirm_renewal(&FixedAccounts(vec![account]), &claude, &home, &before)
        .await
        .unwrap();
    let error = confirm_renewal(&FixedAccounts(Vec::new()), &claude, &home, &before)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("did not renew"), "{error}");
}

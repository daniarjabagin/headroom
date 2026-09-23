use super::super::test_support::{fake_gh_printing_stored_tokens, sign_in};
use super::*;

#[tokio::test]
async fn the_token_of_the_named_account_is_captured() {
    let root = tempfile::tempdir().unwrap();
    let gh = fake_gh_printing_stored_tokens(root.path());
    let config = root.path().join("gh");
    sign_in(
        &config,
        "",
        &[("octocat", "gho_fake_one"), ("mona", "gho_fake_two")],
    );
    let token = gh_token(&gh, &config, "mona").await.unwrap();
    assert_eq!(token.expose(), "gho_fake_two");
    assert_eq!(format!("{token:?}"), "SecretString([redacted])");
}

#[tokio::test]
async fn an_account_without_a_token_must_sign_in_again() {
    let root = tempfile::tempdir().unwrap();
    let gh = fake_gh_printing_stored_tokens(root.path());
    let config = root.path().join("gh");
    sign_in(&config, "", &[]);
    assert_eq!(
        gh_token(&gh, &config, "octocat").await.unwrap_err(),
        ProviderError::SignInExpired
    );
}

#[tokio::test]
async fn a_missing_gh_or_unusable_output_is_a_local_error() {
    let root = tempfile::tempdir().unwrap();
    let missing = gh_token(&root.path().join("no-gh"), root.path(), "octocat").await;
    assert_eq!(
        missing.unwrap_err(),
        ProviderError::LocalData("the GitHub CLI (gh) is not installed".into())
    );
    let gh = fake_gh_printing_stored_tokens(root.path());
    let config = root.path().join("gh");
    sign_in(&config, "", &[("octocat", "two words")]);
    assert!(matches!(
        gh_token(&gh, &config, "octocat").await,
        Err(ProviderError::LocalData(message)) if !message.contains("two words")
    ));
}

#[test]
fn tokens_are_single_printable_words() {
    assert!(is_token("gho_abc123"));
    assert!(!is_token(""));
    assert!(!is_token("a b"));
    assert!(!is_token("tab\there"));
    assert!(!is_token(&"x".repeat(MAX_TOKEN_LEN + 1)));
}

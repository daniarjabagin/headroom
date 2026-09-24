use super::fake::FakeKeychain;
use super::*;

const ITEM: GenericPassword<'static> = GenericPassword {
    service: "io.github.headroom",
    account: Some("tool:0123456789ab"),
};

fn secret(text: &str) -> SecretString {
    SecretString::new(text.to_owned())
}

#[tokio::test]
async fn a_found_item_is_returned_without_the_trailing_newline() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    fake.insert(ITEM.service, ITEM.account, "sk-one");
    let found = fake.security().find(ITEM).await.unwrap();
    assert_eq!(found, Some(secret("sk-one")));
    assert_eq!(
        fake.argv_log(),
        "find-generic-password -s io.github.headroom -a tool:0123456789ab -w\n"
    );
}

#[tokio::test]
async fn exit_44_means_the_item_is_absent() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    let security = fake.security();
    assert_eq!(security.find(ITEM).await.unwrap(), None);
    assert!(!security.delete(ITEM).await.unwrap());
    fake.fail_with(44);
    assert_eq!(security.find(ITEM).await.unwrap(), None);
}

#[tokio::test]
async fn other_exits_are_typed_errors() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    let security = fake.security();
    fake.fail_with(51);
    assert_eq!(security.find(ITEM).await, Err(KeychainError::Denied(51)));
    fake.fail_with(1);
    let error = security.find(ITEM).await.unwrap_err();
    assert_eq!(error, KeychainError::Failed(Some(1)));
    assert_eq!(error.to_string(), "security failed (exit code 1)");
    assert!(!error.is_unavailable());
}

#[tokio::test]
async fn a_hanging_security_times_out() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    fake.hang();
    let timeout = Duration::from_millis(200);
    let error = fake
        .security_with_timeout(timeout)
        .find(ITEM)
        .await
        .unwrap_err();
    assert_eq!(error, KeychainError::TimedOut(timeout));
    assert!(error.is_unavailable());
}

#[tokio::test]
async fn a_missing_program_is_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let security = Security::new(root.path().join("absent"), DEFAULT_TIMEOUT);
    let error = security.find(ITEM).await.unwrap_err();
    assert!(error.is_unavailable(), "{error}");
}

#[tokio::test]
async fn stored_secrets_travel_on_stdin_never_on_argv() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    let security = fake.security();
    let key = secret("sk-\"quoted\" \\ secret");
    security
        .store(ITEM, "Headroom tool API key", &key)
        .await
        .unwrap();
    assert_eq!(
        fake.item(ITEM.service, ITEM.account).as_deref(),
        Some(key.expose())
    );
    let argv = fake.argv_log();
    assert!(!argv.contains("sk-"), "{argv}");
    assert!(!argv.contains(&hex::encode(key.expose())), "{argv}");
    assert!(argv.starts_with("-i\n"), "{argv}");
    assert_eq!(
        fake.stdin_log(),
        format!(
            "add-generic-password -U -s \"io.github.headroom\" -l \"Headroom tool API key\" -a \"tool:0123456789ab\" -X \"{}\"\n",
            hex::encode(key.expose())
        )
    );
    assert_eq!(security.find(ITEM).await.unwrap(), Some(key));
}

#[tokio::test]
async fn a_store_that_does_not_stick_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    fake.fail_with(0);
    let error = fake
        .security()
        .store(ITEM, "label", &secret("sk"))
        .await
        .unwrap_err();
    assert_eq!(error, KeychainError::NotStored);
}

#[test]
fn unsafe_names_and_long_secrets_are_refused() {
    let bad = GenericPassword {
        service: "a\"b",
        account: None,
    };
    assert_eq!(
        add_command(bad, "l", &secret("x")),
        Err(KeychainError::UnsafeValue("service"))
    );
    let newline = GenericPassword {
        service: "ok",
        account: Some("line\nbreak"),
    };
    assert_eq!(
        add_command(newline, "l", &secret("x")),
        Err(KeychainError::UnsafeValue("account"))
    );
    let long = "x".repeat(MAX_COMMAND_LINE);
    assert_eq!(
        add_command(ITEM, "l", &secret(&long)),
        Err(KeychainError::TooLong)
    );
}

#[tokio::test]
async fn deleting_removes_the_item() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    fake.insert(ITEM.service, ITEM.account, "sk");
    assert!(fake.security().delete(ITEM).await.unwrap());
    assert_eq!(fake.item(ITEM.service, ITEM.account), None);
}

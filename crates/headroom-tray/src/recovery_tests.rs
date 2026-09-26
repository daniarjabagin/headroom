use serde_json::{Value, json};

use super::*;

fn account(recovery: Option<Value>) -> Account {
    let mut value = json!({
        "id": "claude:0123456789ab",
        "provider": "claude",
        "provider_name": "Claude",
        "label": null,
        "email": null,
        "plan": null,
        "hidden": false,
        "status": "signed_out",
        "error": {"kind": "sign_in_expired", "message": "Sign-in expired"},
        "updated_at": null,
        "windows": [],
        "balances": [],
        "notices": [],
        "usage_home": "~/.claude"
    });
    if let Some(recovery) = recovery {
        value["recovery"] = recovery;
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn older_daemons_keep_retry_and_the_terminal_sign_in() {
    let old = account(None);
    assert_eq!(old.recovery, RecoveryField::Unreported);
    assert_eq!(
        notice_buttons(&old, true, true),
        [NoticeButton::SignIn, NoticeButton::Retry]
    );
    assert_eq!(notice_buttons(&old, true, false), [NoticeButton::Retry]);
    assert_eq!(notice_buttons(&old, false, true), [NoticeButton::Retry]);
}

#[test]
fn a_null_recovery_offers_no_button() {
    let waiting = account(Some(Value::Null));
    assert_eq!(waiting.recovery, RecoveryField::Wait);
    assert!(notice_buttons(&waiting, true, true).is_empty());
}

#[test]
fn recovery_actions_map_to_buttons() {
    let cases = [
        (json!({"action": "retry"}), NoticeButton::Retry),
        (
            json!({"action": "sign_in", "account_id": "claude:fedcba987654"}),
            NoticeButton::SignInAgain {
                account_id: "claude:fedcba987654".into(),
            },
        ),
        (
            json!({"action": "sign_in"}),
            NoticeButton::SignInAgain {
                account_id: "claude:0123456789ab".into(),
            },
        ),
        (
            json!({"action": "cli_login", "command": " claude auth login --claudeai "}),
            NoticeButton::CopyCommand {
                command: "claude auth login --claudeai".into(),
                primary: true,
            },
        ),
        (
            json!({"action": "cli_login", "command": "  "}),
            NoticeButton::Retry,
        ),
        (json!({"action": "cli_login"}), NoticeButton::Retry),
        (
            json!({"action": "open_portal", "url": "x"}),
            NoticeButton::Retry,
        ),
        (json!("retry"), NoticeButton::Retry),
    ];
    for (recovery, expected) in cases {
        let offered = account(Some(recovery.clone()));
        assert_eq!(
            notice_buttons(&offered, true, true),
            [expected],
            "{recovery}"
        );
    }
}

#[test]
fn cli_login_explains_the_command() {
    let offered = account(Some(
        json!({"action": "cli_login", "command": "codex login"}),
    ));
    assert_eq!(
        cli_login_hint(Lang::En, &offered).as_deref(),
        Some("Run `codex login` in a terminal — Headroom picks it up automatically.")
    );
    assert_eq!(
        cli_login_hint(Lang::Ru, &offered).as_deref(),
        Some("Выполните `codex login` в терминале — Headroom подхватит вход сам.")
    );
    assert_eq!(cli_login_hint(Lang::En, &account(None)), None);
}

#[test]
fn button_models_follow_busy_and_copied_state() {
    let idle = ButtonState::default();
    let busy = ButtonState {
        busy: true,
        copied: false,
    };
    let copied = ButtonState {
        busy: false,
        copied: true,
    };
    let again = NoticeButton::SignInAgain {
        account_id: "a".into(),
    };
    let copy = NoticeButton::CopyCommand {
        command: "codex login".into(),
        primary: true,
    };
    let secondary_copy = NoticeButton::CopyCommand {
        command: "codex login".into(),
        primary: false,
    };
    let cli = NoticeButton::CliSignIn {
        account_id: "a".into(),
    };
    let cases = [
        (Lang::En, NoticeButton::Retry, idle, ("Retry", false, false)),
        (
            Lang::En,
            NoticeButton::Retry,
            busy,
            ("Retrying…", true, false),
        ),
        (
            Lang::En,
            NoticeButton::SignIn,
            busy,
            ("Sign in…", false, true),
        ),
        (
            Lang::En,
            again.clone(),
            busy,
            ("Sign in again…", false, true),
        ),
        (Lang::En, copy.clone(), idle, ("Copy command", false, true)),
        (Lang::En, copy.clone(), copied, ("Copied", false, true)),
        (
            Lang::En,
            secondary_copy.clone(),
            idle,
            ("Copy command", false, false),
        ),
        (Lang::En, secondary_copy, copied, ("Copied", false, false)),
        (Lang::En, cli.clone(), idle, ("Sign in…", false, true)),
        (Lang::Ru, cli, idle, ("Войти…", false, true)),
        (Lang::Ru, again, idle, ("Войти снова…", false, true)),
        (Lang::Ru, copy, idle, ("Скопировать команду", false, true)),
        (
            Lang::Ru,
            NoticeButton::Retry,
            busy,
            ("Повторяем…", true, false),
        ),
    ];
    for (lang, button, state, (label, is_busy, primary)) in cases {
        assert_eq!(
            button_model(lang, &button, state),
            ButtonModel {
                label,
                busy: is_busy,
                primary
            },
            "{button:?} {state:?}"
        );
    }
}

#[test]
fn cli_login_with_an_account_signs_in_through_headroom() {
    let offered = account(Some(json!({
        "action": "cli_login",
        "command": "claude auth login --claudeai",
        "account_id": "claude:0123456789ab"
    })));
    let buttons = notice_buttons(&offered, true, false);
    let sign_in = NoticeButton::CliSignIn {
        account_id: "claude:0123456789ab".into(),
    };
    let copy = NoticeButton::CopyCommand {
        command: "claude auth login --claudeai".into(),
        primary: false,
    };
    assert_eq!(
        buttons,
        [sign_in.clone(), NoticeButton::Retry, copy.clone()]
    );
    assert_eq!(
        button_rows(&buttons),
        [vec![sign_in.clone(), NoticeButton::Retry], vec![copy]]
    );
    let bare = account(Some(
        json!({"action": "cli_login", "account_id": "claude:0123456789ab"}),
    ));
    let buttons = notice_buttons(&bare, true, false);
    assert_eq!(buttons, [sign_in.clone(), NoticeButton::Retry]);
    assert_eq!(button_rows(&buttons), vec![buttons]);
    let blank = account(Some(
        json!({"action": "cli_login", "command": "codex login", "account_id": " "}),
    ));
    assert_eq!(
        notice_buttons(&blank, true, false),
        [NoticeButton::CopyCommand {
            command: "codex login".into(),
            primary: true,
        }]
    );
}

use serde_json::json;

use jiff::tz::TimeZone;

use super::*;
use crate::payload::{Recovery, RecoveryField};

fn locale(lang: Lang) -> Locale {
    Locale::new(lang, TimeZone::UTC)
}

fn account(status: &str, error: Option<(&str, &str)>) -> Account {
    serde_json::from_value(json!({
        "id": "codex:1a2b3c4d5e6f",
        "provider": "codex",
        "provider_name": "Codex",
        "label": null,
        "email": "dev@example.com",
        "plan": "Pro",
        "hidden": false,
        "status": status,
        "error": error.map(|(kind, message)| json!({"kind": kind, "message": message})),
        "updated_at": "2026-09-23T09:58:00Z",
        "windows": [
            {"id": "session", "label": "Session", "used_percent": 38.0, "remaining_percent": 62.0,
             "resets_at": null, "tone": "good", "hidden": false,
             "pace": {"severity": "untracked", "even_pace_percent": null, "projected_percent": null,
                      "spare_percent": null, "runs_out_at": null}},
            {"id": "weekly", "label": "Weekly", "used_percent": 10.0, "remaining_percent": 90.0,
             "resets_at": null, "tone": "good", "hidden": true,
             "pace": {"severity": "untracked", "even_pace_percent": null, "projected_percent": null,
                      "spare_percent": null, "runs_out_at": null}}
        ],
        "balances": [],
        "notices": [{"tone": "neutral", "text": "Weekly limit shared with Codex Cloud"}],
        "usage_home": "~/.codex"
    }))
    .unwrap()
}

#[test]
fn signed_out_blocks_the_card() {
    let signed_out = account("signed_out", Some(("sign_in_expired", "sign-in expired")));
    let CardBody::Blocked(notice) = card_body(&locale(Lang::En), &signed_out, false, true) else {
        panic!("expected a blocking notice");
    };
    assert_eq!(notice.kind, NoticeKind::SignIn);
    assert_eq!(notice.title, "Signed out of Codex");
    assert_eq!(notice.note.as_deref(), Some("sign-in expired"));
    let retrying = account("refreshing", Some(("not_signed_in", "x")));
    assert!(is_signed_out(&retrying));
    assert!(is_retrying(&retrying));
}

#[test]
fn no_subscription_hides_the_plan_and_redundant_notes() {
    let lapsed = account(
        "no_subscription",
        Some(("no_subscription", "No active subscription.")),
    );
    assert_eq!(shown_plan(&lapsed), None);
    let CardBody::Blocked(notice) = card_body(&locale(Lang::En), &lapsed, false, false) else {
        panic!("expected a blocking notice");
    };
    assert_eq!(notice.kind, NoticeKind::Warning);
    assert_eq!(notice.note, None);
    let detailed = AccountError {
        kind: "no_subscription".into(),
        message: "No active ChatGPT subscription (Free plan).".into(),
    };
    assert_eq!(
        subscription_note(Some(&detailed)).as_deref(),
        Some("No active ChatGPT subscription (Free plan).")
    );
}

#[test]
fn errors_add_a_notice_unless_offline() {
    let failed = account("error", Some(("network", "connection refused")));
    let CardBody::Limits {
        alert,
        notices,
        windows,
    } = card_body(&locale(Lang::En), &failed, false, false)
    else {
        panic!("expected limits");
    };
    let alert = alert.unwrap();
    assert_eq!(alert.kind, NoticeKind::Error);
    assert_eq!(alert.title, "Couldn't refresh Codex");
    assert_eq!(alert.buttons, [NoticeButton::Retry]);
    assert_eq!(notices[0].kind, NoticeKind::Info);
    assert!(notices[0].buttons.is_empty());
    assert_eq!(windows.len(), 1);
    assert_eq!(header_status(&failed, false), Some(HeaderStatus::Error));
    assert_eq!(header_status(&failed, true), Some(HeaderStatus::Outdated));
    let CardBody::Limits { alert, notices, .. } =
        card_body(&locale(Lang::En), &failed, true, false)
    else {
        panic!("expected limits");
    };
    assert_eq!(alert, None);
    assert_eq!(notices.len(), 1);
}

#[test]
fn an_incident_mark_replaces_the_error_triangle() {
    use HeaderStatus::{Error, Outdated, Refreshing};
    let cases = [
        (Some(Error), false, Some(Error)),
        (Some(Error), true, None),
        (Some(Refreshing), true, Some(Refreshing)),
        (Some(Outdated), true, Some(Outdated)),
        (None, true, None),
    ];
    for (status, incident, shown) in cases {
        assert_eq!(
            header_mark(status, incident),
            shown,
            "{status:?} {incident}"
        );
    }
}

#[test]
fn first_refresh_shows_a_skeleton() {
    let mut waiting = account("refreshing", None);
    waiting.updated_at = None;
    assert_eq!(
        card_body(&locale(Lang::En), &waiting, false, false),
        CardBody::Skeleton(2)
    );
    assert_eq!(
        header_status(&waiting, false),
        Some(HeaderStatus::Refreshing)
    );
}

#[test]
fn titles_name_the_account_only_when_needed() {
    let first = account("fresh", None);
    let mut second = account("fresh", None);
    second.label = Some("work".into());
    assert!(!shows_name(&first, &[&first]));
    assert!(shows_name(&first, &[&first, &second]));
    assert_eq!(account_title(&first, false), "Codex");
    assert_eq!(account_title(&first, true), "Codex: dev@example.com");
    assert_eq!(account_title(&second, true), "Codex: work");
    assert_eq!(header_status(&first, false), None);
}

fn alert_of(body: CardBody<'_>) -> Option<NoticeView> {
    match body {
        CardBody::Blocked(notice) => Some(notice),
        CardBody::Limits { alert, .. } => alert,
        CardBody::Skeleton(_) => None,
    }
}

#[test]
fn the_error_notice_stays_while_retrying() {
    let cases = [
        ("error", "network", "connection refused"),
        ("error", "account_changed", "Another account is signed in"),
        ("signed_out", "sign_in_expired", "sign-in expired"),
    ];
    for (status, kind, message) in cases {
        let failed = account(status, Some((kind, message)));
        let mut retrying = failed.clone();
        retrying.status = Status::Refreshing;
        let before = alert_of(card_body(&locale(Lang::En), &failed, false, false));
        let during = alert_of(card_body(&locale(Lang::En), &retrying, false, false));
        assert!(before.is_some(), "{kind}");
        assert_eq!(before, during, "{kind}");
        assert!(is_retrying(&retrying));
    }
}

#[test]
fn a_first_refresh_that_failed_keeps_its_notice() {
    let mut retrying = account("refreshing", Some(("timeout", "timed out")));
    retrying.updated_at = None;
    assert!(shows_error_notice(&retrying, false));
    assert!(alert_of(card_body(&locale(Lang::En), &retrying, false, false)).is_some());
    let healthy = account("refreshing", None);
    assert!(!shows_error_notice(&healthy, false));
    let offline = account("refreshing", Some(("network", "offline")));
    assert!(!shows_error_notice(&offline, true));
}

#[test]
fn account_changes_have_their_own_title() {
    let changed = account("error", Some(("account_changed", "x")));
    let alert = alert_of(card_body(&locale(Lang::En), &changed, false, false)).unwrap();
    assert_eq!(alert.title, "Another account is signed in to Codex");
    let alert = alert_of(card_body(&locale(Lang::Ru), &changed, false, false)).unwrap();
    assert_eq!(alert.title, "В Codex выполнен вход в другой аккаунт");
}

#[test]
fn signed_out_notices_follow_the_recovery() {
    let mut signed_out = account("signed_out", Some(("sign_in_expired", "expired")));
    let legacy = alert_of(card_body(&locale(Lang::En), &signed_out, false, true)).unwrap();
    assert_eq!(legacy.buttons, [NoticeButton::SignIn, NoticeButton::Retry]);
    signed_out.recovery = RecoveryField::Offered(Recovery::CliLogin {
        command: "codex login".into(),
        account_id: None,
    });
    let cli = alert_of(card_body(&locale(Lang::En), &signed_out, false, true)).unwrap();
    assert_eq!(
        cli.buttons,
        [NoticeButton::CopyCommand {
            command: "codex login".into(),
            primary: true,
        }]
    );
    assert_eq!(
        cli.detail.as_deref(),
        Some("Run `codex login` in a terminal — Headroom picks it up automatically.")
    );
    signed_out.recovery = RecoveryField::Wait;
    let waiting = alert_of(card_body(&locale(Lang::En), &signed_out, false, true)).unwrap();
    assert!(waiting.buttons.is_empty());
    let lapsed = account("no_subscription", Some(("no_subscription", "x")));
    let legacy_lapsed = alert_of(card_body(&locale(Lang::En), &lapsed, false, false)).unwrap();
    assert_eq!(legacy_lapsed.buttons, [NoticeButton::Retry]);
}

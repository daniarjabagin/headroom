use serde_json::json;

use super::*;

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
    let CardBody::Blocked(notice) = card_body(Lang::En, &signed_out, false) else {
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
    let CardBody::Blocked(notice) = card_body(Lang::En, &lapsed, false) else {
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
    let CardBody::Limits { notices, windows } = card_body(Lang::En, &failed, false) else {
        panic!("expected limits");
    };
    assert_eq!(notices[0].kind, NoticeKind::Error);
    assert_eq!(notices[0].title, "Couldn't refresh Codex");
    assert_eq!(notices[1].kind, NoticeKind::Info);
    assert_eq!(windows.len(), 1);
    assert_eq!(header_status(&failed, false), Some(HeaderStatus::Error));
    assert_eq!(header_status(&failed, true), Some(HeaderStatus::Outdated));
    let CardBody::Limits { notices, .. } = card_body(Lang::En, &failed, true) else {
        panic!("expected limits");
    };
    assert_eq!(notices.len(), 1);
}

#[test]
fn first_refresh_shows_a_skeleton() {
    let mut waiting = account("refreshing", None);
    waiting.updated_at = None;
    assert_eq!(card_body(Lang::En, &waiting, false), CardBody::Skeleton(2));
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

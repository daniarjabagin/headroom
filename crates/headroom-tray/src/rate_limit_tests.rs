use jiff::tz::TimeZone;

use super::*;
use crate::account::{CardBody, HeaderStatus, card_body, header_status, shows_error_notice};
use crate::dates::Clock;
use crate::i18n::Lang;
use crate::notices::NoticeKind;
use crate::payload::Status;
use crate::popup_model::collapse::{Attention, account_attention};

const LIMITED: &str =
    include_str!("../../headroom-daemon/src/state/snapshots/account_rate_limited.json");

fn limited() -> Account {
    serde_json::from_str(LIMITED).unwrap()
}

fn locale(lang: Lang) -> Locale {
    Locale::new(lang, TimeZone::UTC)
}

#[test]
fn a_limited_account_with_data_is_quiet() {
    let account = limited();
    assert!(is_quietly_limited(&account));
    let mut unloaded = account.clone();
    unloaded.updated_at = None;
    assert!(!is_quietly_limited(&unloaded));
    let mut other = account;
    other.error.as_mut().unwrap().kind = "network".into();
    assert!(!is_quietly_limited(&other));
}

#[test]
fn the_note_names_the_next_try_in_the_chosen_clock() {
    let account = limited();
    assert_eq!(
        rate_limit_note(&account, &locale(Lang::En)).as_deref(),
        Some("Provider is limiting requests · next try 10:05")
    );
    assert_eq!(
        rate_limit_note(&account, &locale(Lang::Ru)).as_deref(),
        Some("Провайдер ограничил запросы · повтор в 10:05")
    );
    let twelve = locale(Lang::En).with_clock(Clock::H12);
    assert_eq!(
        rate_limit_note(&account, &twelve).as_deref(),
        Some("Provider is limiting requests · next try 10:05\u{a0}AM")
    );
}

#[test]
fn without_a_next_try_the_note_stays_short() {
    let mut account = limited();
    account.refresh = None;
    assert_eq!(
        rate_limit_note(&account, &locale(Lang::En)).as_deref(),
        Some("Provider is limiting requests")
    );
    account.updated_at = None;
    assert_eq!(rate_limit_note(&account, &locale(Lang::En)), None);
}

#[test]
fn a_quiet_limit_replaces_the_error_notice_with_a_line() {
    let account = limited();
    let CardBody::Limits {
        alert,
        notices,
        windows,
    } = card_body(&locale(Lang::En), &account, false, false)
    else {
        panic!("expected limits");
    };
    assert_eq!(alert, None);
    assert_eq!(notices[0].kind, NoticeKind::Info);
    assert_eq!(
        notices[0].title,
        "Provider is limiting requests · next try 10:05"
    );
    assert_eq!(windows.len(), 1);
    assert_eq!(header_status(&account, false), Some(HeaderStatus::Outdated));
}

#[test]
fn a_quiet_limit_raises_no_notice_while_retrying() {
    let mut account = limited();
    account.status = Status::Refreshing;
    assert!(!shows_error_notice(&account, false));
    assert_ne!(account_attention(&account, false), Some(Attention::Notice));
}

#[test]
fn a_limit_without_data_keeps_the_error_notice() {
    let mut account = limited();
    account.status = Status::Error;
    account.updated_at = None;
    assert!(shows_error_notice(&account, false));
    assert_eq!(account_attention(&account, false), Some(Attention::Notice));
    let CardBody::Limits { alert, notices, .. } =
        card_body(&locale(Lang::En), &account, false, false)
    else {
        panic!("expected limits");
    };
    assert_eq!(alert.unwrap().kind, NoticeKind::Error);
    assert!(notices.is_empty());
}

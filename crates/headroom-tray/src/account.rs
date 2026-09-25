use crate::i18n::{Lang, fill};
use crate::notices::{NoticeKind, notice_text};
use crate::payload::{Account, AccountError, Status, Window};
use crate::recovery::{NoticeButton, cli_login_hint, notice_buttons};

const NO_SUBSCRIPTION: &str = "no_subscription";
const NETWORK: &str = "network";
const ACCOUNT_CHANGED: &str = "account_changed";
const SIGN_IN_ERRORS: [&str; 2] = ["not_signed_in", "sign_in_expired"];
const SKELETON_ROWS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderStatus {
    Refreshing,
    Outdated,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeView {
    pub kind: NoticeKind,
    pub title: String,
    pub detail: Option<String>,
    pub note: Option<String>,
    pub buttons: Vec<NoticeButton>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CardBody<'a> {
    Blocked(NoticeView),
    Skeleton(usize),
    Limits {
        alert: Option<NoticeView>,
        notices: Vec<NoticeView>,
        windows: Vec<&'a Window>,
    },
}

fn error_kind(account: &Account) -> Option<&str> {
    account.error.as_ref().map(|error| error.kind.as_str())
}

#[must_use]
pub fn lacks_subscription(account: &Account) -> bool {
    account.status == Status::NoSubscription
        || (account.status == Status::Refreshing && error_kind(account) == Some(NO_SUBSCRIPTION))
}

#[must_use]
pub fn is_signed_out(account: &Account) -> bool {
    account.status == Status::SignedOut
        || (account.status == Status::Refreshing
            && error_kind(account).is_some_and(|kind| SIGN_IN_ERRORS.contains(&kind)))
}

#[must_use]
pub fn is_retrying(account: &Account) -> bool {
    account.status == Status::Refreshing
}

fn failed_offline(account: &Account, offline: bool) -> bool {
    offline && error_kind(account) == Some(NETWORK)
}

#[must_use]
pub fn header_status(account: &Account, offline: bool) -> Option<HeaderStatus> {
    match account.status {
        Status::Refreshing => Some(HeaderStatus::Refreshing),
        Status::Stale => Some(HeaderStatus::Outdated),
        Status::Error if failed_offline(account, offline) => Some(HeaderStatus::Outdated),
        Status::Error => Some(HeaderStatus::Error),
        _ => None,
    }
}

#[must_use]
pub fn shows_name(account: &Account, accounts: &[&Account]) -> bool {
    accounts
        .iter()
        .filter(|other| other.provider == account.provider)
        .count()
        > 1
}

#[must_use]
pub fn account_title(account: &Account, show_name: bool) -> String {
    let who = account.label.as_ref().or(account.email.as_ref());
    match who {
        Some(who) if show_name => format!("{}: {who}", account.provider_name),
        _ => account.provider_name.clone(),
    }
}

#[must_use]
pub fn shown_plan(account: &Account) -> Option<&str> {
    account
        .plan
        .as_deref()
        .filter(|_| !lacks_subscription(account))
}

fn normalized(text: &str) -> String {
    let spaced: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[must_use]
pub fn subscription_note(error: Option<&AccountError>) -> Option<String> {
    let error = error?;
    let text = normalized(&error.message);
    let redundant = error.message == error.kind
        || text.is_empty()
        || text == "no active subscription"
        || text == normalized(NO_SUBSCRIPTION);
    (!redundant).then(|| error.message.clone())
}

fn signed_out_notice(lang: Lang, account: &Account, terminal_sign_in: bool) -> NoticeView {
    let detail = cli_login_hint(lang, account).unwrap_or_else(|| {
        lang.tr("Sign in again to keep this account up to date.")
            .to_owned()
    });
    NoticeView {
        kind: NoticeKind::SignIn,
        title: fill(
            lang.tr("Signed out of {provider}"),
            &[("provider", &account.provider_name)],
        ),
        detail: Some(detail),
        note: account.error.as_ref().map(|error| error.message.clone()),
        buttons: notice_buttons(account, true, terminal_sign_in),
    }
}

fn no_subscription_notice(lang: Lang, account: &Account) -> NoticeView {
    NoticeView {
        kind: NoticeKind::Warning,
        title: lang.tr("No active subscription").to_owned(),
        detail: Some(
            lang.tr(
                "Limits aren't available for this account. Renew the plan or sign in with another account.",
            )
            .to_owned(),
        ),
        note: subscription_note(account.error.as_ref()),
        buttons: notice_buttons(account, false, false),
    }
}

fn error_title(lang: Lang, account: &Account) -> String {
    let template = if error_kind(account) == Some(ACCOUNT_CHANGED) {
        "Another account is signed in to {provider}"
    } else {
        "Couldn't refresh {provider}"
    };
    fill(lang.tr(template), &[("provider", &account.provider_name)])
}

fn error_notice(lang: Lang, account: &Account) -> NoticeView {
    NoticeView {
        kind: NoticeKind::Error,
        title: error_title(lang, account),
        detail: account.error.as_ref().map(|error| error.message.clone()),
        note: cli_login_hint(lang, account),
        buttons: notice_buttons(account, false, false),
    }
}

fn daemon_notices(lang: Lang, account: &Account) -> impl Iterator<Item = NoticeView> + '_ {
    account.notices.iter().map(move |notice| NoticeView {
        kind: NoticeKind::from_tone(notice.tone),
        title: notice_text(lang, &notice.text),
        detail: None,
        note: None,
        buttons: Vec::new(),
    })
}

#[must_use]
pub fn shown_windows(account: &Account) -> Vec<&Window> {
    account
        .windows
        .iter()
        .filter(|window| !window.hidden)
        .collect()
}

fn awaiting_first_data(account: &Account) -> bool {
    account.status == Status::Refreshing && account.updated_at.is_none()
}

#[must_use]
pub fn shows_error_notice(account: &Account, offline: bool) -> bool {
    let reported = match account.status {
        Status::Error => true,
        Status::Refreshing => account.error.is_some(),
        _ => false,
    };
    reported && !failed_offline(account, offline)
}

#[must_use]
pub fn card_body(
    lang: Lang,
    account: &Account,
    offline: bool,
    terminal_sign_in: bool,
) -> CardBody<'_> {
    if is_signed_out(account) {
        return CardBody::Blocked(signed_out_notice(lang, account, terminal_sign_in));
    }
    if lacks_subscription(account) {
        return CardBody::Blocked(no_subscription_notice(lang, account));
    }
    let failed = shows_error_notice(account, offline);
    if awaiting_first_data(account) && !failed {
        return CardBody::Skeleton(SKELETON_ROWS.max(account.windows.len()));
    }
    CardBody::Limits {
        alert: failed.then(|| error_notice(lang, account)),
        notices: daemon_notices(lang, account).collect(),
        windows: shown_windows(account),
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;

use crate::dates::{Locale, clock_time};
use crate::i18n::fill;
use crate::payload::Account;

const RATE_LIMITED: &str = "rate_limited";

#[must_use]
pub fn is_quietly_limited(account: &Account) -> bool {
    account
        .error
        .as_ref()
        .is_some_and(|error| error.kind == RATE_LIMITED)
        && account.updated_at.is_some()
}

#[must_use]
pub fn rate_limit_note(account: &Account, locale: &Locale) -> Option<String> {
    if !is_quietly_limited(account) {
        return None;
    }
    let lang = locale.lang;
    let note = match account.refresh.as_ref().and_then(|refresh| refresh.next_at) {
        Some(next) => fill(
            lang.tr("Provider is limiting requests · next try {time}"),
            &[("time", &clock_time(next, locale))],
        ),
        None => lang.tr("Provider is limiting requests").to_owned(),
    };
    Some(note)
}

#[cfg(test)]
#[path = "rate_limit_tests.rs"]
mod tests;

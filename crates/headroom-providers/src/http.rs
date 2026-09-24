use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::redirect::{Action, Attempt, Policy};
use reqwest::{Client, Url};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const POOL_IDLE_PER_HOST: usize = 1;
const MAX_REDIRECTS: usize = 5;
const MAX_RETRY_AFTER: SignedDuration = SignedDuration::from_hours(24);
const RELEASE_HOST: &str = "github.com";
const RELEASE_ASSET_DOMAIN: &str = ".githubusercontent.com";

#[derive(Debug, thiserror::Error)]
enum RedirectRefused {
    #[error("refused a redirect to another origin")]
    CrossOrigin,
    #[error("stopped after {MAX_REDIRECTS} redirects")]
    TooMany,
}

/// Builds the one HTTP client a process shares: TLS roots and the connection pool are loaded once.
pub fn client() -> Result<Client, ProviderError> {
    Client::builder()
        .user_agent(concat!("headroom/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .pool_max_idle_per_host(POOL_IDLE_PER_HOST)
        .redirect(Policy::custom(follow_same_origin))
        .build()
        .map_err(|error| ProviderError::Network(error.without_url().to_string()))
}

fn follow_same_origin(attempt: Attempt<'_>) -> Action {
    let previous = attempt.previous();
    if previous.len() > MAX_REDIRECTS {
        return attempt.error(RedirectRefused::TooMany);
    }
    match previous.last() {
        Some(from) if redirect_allowed(from, attempt.url()) => attempt.follow(),
        _ => attempt.error(RedirectRefused::CrossOrigin),
    }
}

fn redirect_allowed(from: &Url, to: &Url) -> bool {
    from.origin() == to.origin() || is_release_asset_hop(from, to)
}

fn is_release_asset_hop(from: &Url, to: &Url) -> bool {
    let https = from.scheme() == "https" && to.scheme() == "https";
    let from_release = from.host_str() == Some(RELEASE_HOST);
    let to_assets = to
        .host_str()
        .is_some_and(|host| host.ends_with(RELEASE_ASSET_DOMAIN));
    https && from_release && to_assets
}

/// The `Retry-After` wait in delay-seconds or as an HTTP date, rounded up to whole seconds.
#[must_use]
pub fn retry_after(headers: &HeaderMap, now: Timestamp) -> Option<SignedDuration> {
    let value = retry_after_text(headers)?;
    delay_seconds(value)
        .or_else(|| wait_until_date(value, now))
        .filter(within_reason)
}

/// The `Retry-After` wait for servers that only send delay-seconds.
#[must_use]
pub fn retry_after_seconds(headers: &HeaderMap) -> Option<SignedDuration> {
    delay_seconds(retry_after_text(headers)?).filter(within_reason)
}

fn within_reason(wait: &SignedDuration) -> bool {
    *wait <= MAX_RETRY_AFTER
}

fn retry_after_text(headers: &HeaderMap) -> Option<&str> {
    headers.get(RETRY_AFTER)?.to_str().ok().map(str::trim)
}

fn delay_seconds(value: &str) -> Option<SignedDuration> {
    let seconds = value.parse::<u32>().ok()?;
    Some(SignedDuration::from_secs(i64::from(seconds)))
}

fn wait_until_date(value: &str, now: Timestamp) -> Option<SignedDuration> {
    let at = jiff::fmt::rfc2822::parse(value).ok()?.timestamp();
    let wait = at.duration_since(now).max(SignedDuration::ZERO);
    let partial = i64::from(wait.subsec_nanos() > 0);
    Some(SignedDuration::from_secs(wait.as_secs() + partial))
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;

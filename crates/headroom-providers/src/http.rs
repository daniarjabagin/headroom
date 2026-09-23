use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::Client;
use reqwest::header::{HeaderMap, RETRY_AFTER};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const POOL_IDLE_PER_HOST: usize = 1;

/// Builds the one HTTP client a process shares: TLS roots and the connection pool are loaded once.
pub fn client() -> Result<Client, ProviderError> {
    Client::builder()
        .user_agent(concat!("headroom/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .pool_max_idle_per_host(POOL_IDLE_PER_HOST)
        .build()
        .map_err(|error| ProviderError::Network(error.without_url().to_string()))
}

/// The `Retry-After` wait in delay-seconds or as an HTTP date, rounded up to whole seconds.
#[must_use]
pub fn retry_after(headers: &HeaderMap, now: Timestamp) -> Option<SignedDuration> {
    let value = retry_after_text(headers)?;
    delay_seconds(value).or_else(|| wait_until_date(value, now))
}

/// The `Retry-After` wait for servers that only send delay-seconds.
#[must_use]
pub fn retry_after_seconds(headers: &HeaderMap) -> Option<SignedDuration> {
    delay_seconds(retry_after_text(headers)?)
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

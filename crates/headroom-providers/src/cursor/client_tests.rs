use jiff::SignedDuration;
use reqwest::header::{HeaderValue, RETRY_AFTER};

use super::*;
use crate::cursor::test_support::{fixed_now, valid_token};

fn headers_with_retry(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_str(value).unwrap());
    headers
}

#[test]
fn statuses_map_to_provider_errors() {
    let none = HeaderMap::new();
    let now = fixed_now();
    assert_eq!(
        status_error(StatusCode::UNAUTHORIZED, &none, now),
        ProviderError::SignInExpired
    );
    assert_eq!(
        status_error(StatusCode::FORBIDDEN, &none, now),
        ProviderError::SignInExpired
    );
    assert_eq!(
        status_error(StatusCode::TOO_MANY_REQUESTS, &none, now),
        ProviderError::RateLimited { retry_after: None }
    );
    assert!(matches!(
        status_error(StatusCode::BAD_GATEWAY, &none, now),
        ProviderError::Network(_)
    ));
    assert!(matches!(
        status_error(StatusCode::NOT_FOUND, &none, now),
        ProviderError::InvalidResponse(_)
    ));
}

#[test]
fn rate_limits_carry_retry_after_seconds_and_dates() {
    let now = fixed_now();
    assert_eq!(
        status_error(
            StatusCode::TOO_MANY_REQUESTS,
            &headers_with_retry("120"),
            now
        ),
        ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(120))
        }
    );
    assert_eq!(
        status_error(
            StatusCode::TOO_MANY_REQUESTS,
            &headers_with_retry("Wed, 23 Sep 2026 10:01:00 GMT"),
            now
        ),
        ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(60))
        }
    );
}

#[test]
fn session_cookie_uses_the_user_id_after_the_connection() {
    let secret = valid_token("google-oauth2|user_01");
    let token = AccessToken::new(secret.clone());
    let session = Session::new("google-oauth2|user_01", &token);
    assert_eq!(session.user_id, "user_01");
    assert_eq!(
        session.cookie,
        format!("WorkosCursorSessionToken=user_01%3A%3A{secret}")
    );
    assert!(!format!("{session:?}").contains(&secret));
}

#[test]
fn unparseable_bodies_are_invalid_responses() {
    let error = parse::<RawPeriodUsage>(b"<html>").unwrap_err();
    assert!(matches!(error, ProviderError::InvalidResponse(_)));
    let wrong_shape = parse::<RawPeriodUsage>(br#"{"planUsage":"none"}"#).unwrap_err();
    assert!(matches!(wrong_shape, ProviderError::InvalidResponse(_)));
}

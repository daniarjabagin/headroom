use reqwest::header::HeaderValue;

use super::*;

fn now() -> Timestamp {
    "2026-09-23T10:00:00.250Z".parse().unwrap()
}

fn headers(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_str(value).unwrap());
    headers
}

fn seconds(value: i64) -> SignedDuration {
    SignedDuration::from_secs(value)
}

#[test]
fn the_shared_client_builds() {
    assert!(client().is_ok());
}

#[test]
fn retry_after_reads_delay_seconds_and_http_dates() {
    let cases = [
        ("120", Some(seconds(120))),
        (" 7 ", Some(seconds(7))),
        ("0", Some(seconds(0))),
        ("Wed, 23 Sep 2026 10:01:30 GMT", Some(seconds(90))),
        ("Wed, 23 Sep 2026 10:00:10 +0000", Some(seconds(10))),
        ("Wed, 23 Sep 2026 09:00:00 GMT", Some(seconds(0))),
        ("-5", None),
        ("1.5", None),
        ("soon", None),
        ("", None),
    ];
    for (value, expected) in cases {
        assert_eq!(retry_after(&headers(value), now()), expected, "{value:?}");
    }
}

#[test]
fn retry_after_seconds_ignores_dates() {
    assert_eq!(retry_after_seconds(&headers("45")), Some(seconds(45)));
    assert_eq!(
        retry_after_seconds(&headers("Wed, 23 Sep 2026 10:01:30 GMT")),
        None
    );
    assert_eq!(retry_after_seconds(&headers("-1")), None);
}

#[test]
fn a_missing_or_binary_header_has_no_wait() {
    assert_eq!(retry_after(&HeaderMap::new(), now()), None);
    assert_eq!(retry_after_seconds(&HeaderMap::new()), None);
    let mut binary = HeaderMap::new();
    binary.insert(RETRY_AFTER, HeaderValue::from_bytes(&[0xff]).unwrap());
    assert_eq!(retry_after(&binary, now()), None);
}

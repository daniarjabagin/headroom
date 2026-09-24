use reqwest::header::HeaderValue;
use wiremock::matchers::{any, body_string, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn absurd_retry_after_values_are_ignored() {
    let cases = [
        ("86400", Some(seconds(86_400))),
        ("86401", None),
        ("4294967295", None),
        ("Fri, 31 Dec 9999 23:59:59 GMT", None),
        ("Thu, 24 Sep 2026 10:00:00 GMT", Some(seconds(86_400))),
    ];
    for (value, expected) in cases {
        assert_eq!(retry_after(&headers(value), now()), expected, "{value:?}");
    }
    assert_eq!(retry_after_seconds(&headers("4294967295")), None);
}

#[test]
fn redirects_stay_on_the_same_origin() {
    let cases = [
        (
            "https://api.example.com/a",
            "https://api.example.com/b",
            true,
        ),
        (
            "https://api.example.com/a",
            "https://api.example.com:443/b",
            true,
        ),
        ("http://127.0.0.1:8080/a", "http://127.0.0.1:8080/b", true),
        (
            "https://api.example.com/a",
            "http://api.example.com/b",
            false,
        ),
        (
            "https://api.example.com/a",
            "https://evil.example.com/b",
            false,
        ),
        (
            "https://api.example.com/a",
            "https://api.example.com:8443/b",
            false,
        ),
        ("http://127.0.0.1:8080/a", "http://127.0.0.1:8081/b", false),
        ("http://127.0.0.1:8080/a", "http://localhost:8080/b", false),
    ];
    for (from, to, allowed) in cases {
        assert_eq!(
            redirect_allowed(&url(from), &url(to)),
            allowed,
            "{from} -> {to}"
        );
    }
}

#[test]
fn release_downloads_get_no_cross_origin_exception() {
    let release = "https://github.com/o/r/releases/download/v1/a.tar.gz";
    for to in [
        "https://release-assets.githubusercontent.com/x",
        "https://objects.githubusercontent.com/x",
    ] {
        assert!(!redirect_allowed(&url(release), &url(to)), "{to}");
    }
}

#[tokio::test]
async fn a_cross_origin_redirect_is_refused_and_the_target_receives_nothing() {
    let origin = MockServer::start().await;
    let elsewhere = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(308)
                .insert_header("location", format!("{}/steal", elsewhere.uri())),
        )
        .mount(&origin)
        .await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&elsewhere)
        .await;
    let result = client()
        .unwrap()
        .post(format!("{}/token", origin.uri()))
        .body("refresh_token=secret")
        .send()
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn a_same_origin_redirect_is_followed_with_its_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/new"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/new"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .expect(1)
        .mount(&server)
        .await;
    let response = client()
        .unwrap()
        .post(format!("{}/old", server.uri()))
        .body("payload")
        .send()
        .await
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn a_redirect_loop_stops_after_five_hops() {
    let server = MockServer::start().await;
    Mock::given(path("/loop"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/loop"))
        .expect(6)
        .mount(&server)
        .await;
    let result = client()
        .unwrap()
        .get(format!("{}/loop", server.uri()))
        .send()
        .await;
    assert!(result.is_err());
}

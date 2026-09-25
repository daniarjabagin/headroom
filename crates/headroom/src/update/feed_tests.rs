use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const FIXTURE: &str =
    include_str!("../../../headroom-daemon/src/update/fixtures/release_latest.json");

fn feed_for(server: &MockServer) -> GithubFeed {
    GithubFeed::new(
        format!("{}/releases/latest", server.uri()),
        Origins::plain_http("127.0.0.1"),
    )
    .unwrap()
}

#[tokio::test]
async fn a_new_release_is_returned_with_its_etag() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/releases/latest"))
        .and(header("user-agent", AGENT))
        .and(header("accept", GITHUB_JSON))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "W/\"e1\"")
                .set_body_string(FIXTURE),
        )
        .expect(1)
        .mount(&server)
        .await;
    let answer = feed_for(&server).latest(None).await.unwrap();
    assert_eq!(
        answer,
        FeedResponse::Modified {
            body: FIXTURE.into(),
            etag: Some("W/\"e1\"".into())
        }
    );
}

#[tokio::test]
async fn the_etag_is_sent_and_not_modified_is_recognised() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("if-none-match", "W/\"e1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    let answer = feed_for(&server).latest(Some("W/\"e1\"")).await;
    assert_eq!(answer, Ok(FeedResponse::NotModified));
}

#[tokio::test]
async fn rate_limits_carry_the_wait_and_other_errors_fail() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header_exists("if-none-match"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let feed = feed_for(&server);
    assert_eq!(
        feed.latest(Some("x")).await,
        Err(FeedError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(120))
        })
    );
    assert!(matches!(feed.latest(None).await, Err(FeedError::Failed(_))));
}

#[test]
fn the_rate_limit_reset_header_gives_the_wait() {
    let now: Timestamp = "2026-09-23T10:00:00Z".parse().unwrap();
    let mut headers = HeaderMap::new();
    let reset = now.as_second() + 1_800;
    headers.insert(RATE_LIMIT_RESET, reset.to_string().parse().unwrap());
    assert_eq!(
        rate_limit_wait(&headers, now),
        Some(SignedDuration::from_mins(30))
    );
    assert_eq!(rate_limit_wait(&HeaderMap::new(), now), None);
}

#[tokio::test]
async fn the_cli_reads_the_release_and_downloads_assets() {
    let server = MockServer::start().await;
    Mock::given(path("/releases/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .mount(&server)
        .await;
    Mock::given(path("/asset"))
        .and(header("user-agent", AGENT))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes".to_vec()))
        .mount(&server)
        .await;
    let feed = feed_for(&server);
    assert_eq!(feed.release().await.unwrap().tag_name, "v0.5.0");
    let asset = format!("{}/asset", server.uri());
    assert_eq!(feed.download(&asset, 5).await.unwrap(), b"bytes");
    let missing = format!("{}/missing", server.uri());
    assert!(feed.download(&missing, 5).await.is_err());
}

#[tokio::test]
async fn downloads_outside_the_allowed_origins_are_refused_without_a_request() {
    let server = MockServer::start().await;
    let feed = feed_for(&server);
    let port = server.address().port();
    for url in [
        format!("http://localhost:{port}/asset"),
        format!("https://127.0.0.1:{port}/asset"),
        "file:///etc/passwd".to_owned(),
    ] {
        let error = feed.download(&url, 64).await.unwrap_err().to_string();
        assert!(error.starts_with("refusing to download"), "{error}");
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn redirects_are_followed_only_to_allowed_origins() {
    let server = MockServer::start().await;
    let port = server.address().port();
    let redirect = |to: String| ResponseTemplate::new(302).insert_header("location", to);
    Mock::given(path("/inside"))
        .respond_with(redirect(format!("http://127.0.0.1:{port}/asset")))
        .mount(&server)
        .await;
    Mock::given(path("/outside"))
        .respond_with(redirect(format!("http://localhost:{port}/asset")))
        .mount(&server)
        .await;
    Mock::given(path("/asset"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes".to_vec()))
        .mount(&server)
        .await;
    let feed = feed_for(&server);
    let inside = format!("{}/inside", server.uri());
    assert_eq!(feed.download(&inside, 5).await.unwrap(), b"bytes");
    let outside = format!("{}/outside", server.uri());
    assert!(feed.download(&outside, 5).await.is_err());
    let assets = server.received_requests().await.unwrap();
    assert_eq!(
        assets.iter().filter(|r| r.url.path() == "/asset").count(),
        1
    );
}

#[tokio::test]
async fn responses_over_the_size_cap_are_refused() {
    let server = MockServer::start().await;
    Mock::given(path("/asset"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 6]))
        .mount(&server)
        .await;
    let huge_release = format!("{{\"pad\":\"{}\"}}", "x".repeat(API_BODY_LIMIT));
    Mock::given(path("/releases/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_string(huge_release))
        .mount(&server)
        .await;
    let feed = feed_for(&server);
    let asset = format!("{}/asset", server.uri());
    let error = format!("{:#}", feed.download(&asset, 5).await.unwrap_err());
    assert!(error.contains("larger than 5 bytes"), "{error}");
    assert!(feed.release().await.is_err());
    assert!(matches!(feed.latest(None).await, Err(FeedError::Failed(_))));
}

#[test]
fn release_checks_finish_before_a_d_bus_call_times_out() {
    let feed = GithubFeed::new(
        "http://127.0.0.1:9/releases/latest",
        Origins::plain_http("127.0.0.1"),
    )
    .unwrap();
    let request = feed.api(Some("W/\"e1\"")).build().unwrap();
    assert_eq!(request.timeout(), Some(&API_TIMEOUT));
    assert!(API_TIMEOUT <= Duration::from_secs(20));
    assert!(CONNECT_TIMEOUT < API_TIMEOUT);
}

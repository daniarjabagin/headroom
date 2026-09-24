use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const FIXTURE: &str =
    include_str!("../../../headroom-daemon/src/update/fixtures/release_latest.json");

fn feed_for(server: &MockServer) -> GithubFeed {
    GithubFeed::new(Client::new(), format!("{}/releases/latest", server.uri()))
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
    assert_eq!(feed.download(&asset).await.unwrap(), b"bytes");
    let missing = format!("{}/missing", server.uri());
    assert!(feed.download(&missing).await.is_err());
}

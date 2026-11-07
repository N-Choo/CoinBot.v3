use std::time::Duration;

use chrono::{TimeZone, Utc};
use signal_gateway::agent_client::{AgentClient, OpenCodeClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn creates_session_sends_message_and_concatenates_text() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "parts": [
                {"type": "text", "text": "hello"},
                {"type": "tool", "text": "ignored"},
                {"type": "text", "text": "world"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = OpenCodeClient::new(
        server.uri(),
        "opencode",
        None,
        "trade-agent",
        None,
        true,
        Duration::from_secs(5),
    )
    .unwrap();

    let text = client
        .predict(
            "BTC-USDT",
            Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(text, "hello\nworld");
}

#[tokio::test]
async fn maps_session_errors_to_network() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = OpenCodeClient::new(
        server.uri(),
        "opencode",
        None,
        "trade-agent",
        None,
        true,
        Duration::from_secs(5),
    )
    .unwrap();
    let err = client
        .predict(
            "BTC-USDT",
            Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, share::ProcessError::Network(_)));
}

#[tokio::test]
async fn maps_message_errors_to_network() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = OpenCodeClient::new(
        server.uri(),
        "opencode",
        None,
        "trade-agent",
        None,
        true,
        Duration::from_secs(5),
    )
    .unwrap();
    let err = client
        .predict(
            "BTC-USDT",
            Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, share::ProcessError::Network(_)));
}

#[tokio::test]
async fn rejects_empty_text_parts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"parts": []})))
        .mount(&server)
        .await;

    let client = OpenCodeClient::new(
        server.uri(),
        "opencode",
        None,
        "trade-agent",
        None,
        true,
        Duration::from_secs(5),
    )
    .unwrap();
    let err = client
        .predict(
            "BTC-USDT",
            Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, share::ProcessError::Internal(_)));
}

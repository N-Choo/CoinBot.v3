use std::time::Duration;

use chrono::{TimeZone, Utc};
use signal_gateway::agent_client::{AgentClient, OpenCodeClient};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(
    server: &MockServer,
    password: Option<&str>,
    model: Option<&str>,
    delete_sessions: bool,
) -> OpenCodeClient {
    OpenCodeClient::new(
        server.uri(),
        "opencode",
        password.map(str::to_string),
        "trade-agent",
        model.map(str::to_string),
        delete_sessions,
        Duration::from_secs(5),
    )
    .unwrap()
}

fn candle() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap()
}

#[tokio::test]
async fn creates_session_sends_message_and_concatenates_text() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/session"))
        .and(body_string_contains(
            "signal-gateway BTC-USDT 2026-11-07T04:00:00+00:00",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .and(body_string_contains("\"agent\":\"trade-agent\""))
        .and(body_string_contains("\"type\":\"text\""))
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

    let text = client(&server, None, None, true)
        .predict("BTC-USDT", candle())
        .await
        .unwrap();
    assert_eq!(text, "hello\nworld");
}

#[tokio::test]
async fn sends_basic_auth_and_model_object() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .and(header("authorization", "Basic b3BlbmNvZGU6c2VjcmV0"))
        .and(body_string_contains("\"providerID\":\"opencode\""))
        .and(body_string_contains("\"modelID\":\"gpt-5.5\""))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"parts": [{"type": "text", "text": "ok"}]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let text = client(&server, Some("secret"), Some("opencode/gpt-5.5"), true)
        .predict("BTC-USDT", candle())
        .await
        .unwrap();
    assert_eq!(text, "ok");
}

#[tokio::test]
async fn deletes_session_when_message_fails() {
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
    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let err = client(&server, None, None, true)
        .predict("BTC-USDT", candle())
        .await
        .unwrap_err();
    assert!(matches!(err, share::ProcessError::Network(_)));
}

#[tokio::test]
async fn does_not_delete_when_disabled() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"parts": [{"type": "text", "text": "ok"}]})),
        )
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let text = client(&server, None, None, false)
        .predict("BTC-USDT", candle())
        .await
        .unwrap();
    assert_eq!(text, "ok");
}

#[tokio::test]
async fn maps_session_errors_to_network() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let err = client(&server, None, None, true)
        .predict("BTC-USDT", candle())
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
    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let err = client(&server, None, None, true)
        .predict("BTC-USDT", candle())
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
    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let err = client(&server, None, None, true)
        .predict("BTC-USDT", candle())
        .await
        .unwrap_err();
    assert!(matches!(err, share::ProcessError::Internal(_)));
}

#[tokio::test]
async fn treats_null_parts_as_empty() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"parts": null})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let err = client(&server, None, None, true)
        .predict("BTC-USDT", candle())
        .await
        .unwrap_err();
    assert!(matches!(err, share::ProcessError::Internal(_)));
}

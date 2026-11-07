use std::time::Duration;

use share::ProcessError;
use signal_gateway::price::{KuCoinPrice, LastPriceSource};
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn returns_last_price_from_kucoin() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(query_param("symbol", "BTC-USDT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data": {"price": "68123.4"}})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = KuCoinPrice::new(server.uri(), Duration::from_secs(5)).unwrap();
    assert_eq!(client.last_price("BTC-USDT").await.unwrap(), 68123.4);
}

#[tokio::test]
async fn maps_http_errors_to_network() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = KuCoinPrice::new(server.uri(), Duration::from_secs(5)).unwrap();
    let err = client.last_price("BTC-USDT").await.unwrap_err();
    assert!(matches!(err, ProcessError::Network(_)));
}

#[tokio::test]
async fn maps_bad_body_to_internal() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let client = KuCoinPrice::new(server.uri(), Duration::from_secs(5)).unwrap();
    let err = client.last_price("BTC-USDT").await.unwrap_err();
    assert!(matches!(err, ProcessError::Internal(_)));
}

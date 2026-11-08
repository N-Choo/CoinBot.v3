//! HTTP client for the opencode server and prompt construction.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use share::ProcessError;

/// Calls the opencode server and returns the trade-agent's text response.
#[async_trait]
pub trait AgentClient: Send + Sync {
    /// Ask the agent for a prediction on one symbol.
    async fn predict(
        &self,
        symbol: &str,
        candle_open: DateTime<Utc>,
    ) -> Result<String, ProcessError>;
}

#[derive(Serialize)]
struct NewSession<'a> {
    title: &'a str,
}

#[derive(Deserialize)]
struct Session {
    id: String,
}

#[derive(Debug, Clone)]
struct ModelRef {
    provider_id: String,
    model_id: String,
}

#[derive(Serialize)]
struct ModelRefBody<'a> {
    #[serde(rename = "providerID")]
    provider_id: &'a str,
    #[serde(rename = "modelID")]
    model_id: &'a str,
}

#[derive(Serialize)]
struct MessageRequest<'a> {
    agent: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<ModelRefBody<'a>>,
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
}

#[derive(Deserialize)]
struct MessageResponse {
    #[serde(default)]
    parts: Option<Vec<MessagePart>>,
}

#[derive(Deserialize)]
struct MessagePart {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

/// HTTP client for an `opencode serve` instance.
pub struct OpenCodeClient {
    base_url: String,
    username: String,
    password: Option<String>,
    agent: String,
    model: Option<ModelRef>,
    delete_sessions: bool,
    client: reqwest::Client,
}

impl OpenCodeClient {
    /// Build the client, validating the base URL and optional model spec.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: Option<String>,
        agent: impl Into<String>,
        model: Option<String>,
        delete_sessions: bool,
        timeout: Duration,
    ) -> Result<Self, ProcessError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        reqwest::Url::parse(&base_url).map_err(|e| {
            ProcessError::InvalidConfig(format!("invalid OPENCODE_BASE_URL {base_url:?}: {e}"))
        })?;

        let model = match model {
            Some(spec) => {
                let (provider_id, model_id) = spec.split_once('/').ok_or_else(|| {
                    ProcessError::InvalidConfig(format!(
                        "OPENCODE_MODEL must be provider/model, got {spec:?}"
                    ))
                })?;
                // Exactly one slash: `split_once` leaves any extra slashes in `model_id`.
                if provider_id.is_empty() || model_id.is_empty() || model_id.contains('/') {
                    return Err(ProcessError::InvalidConfig(format!(
                        "OPENCODE_MODEL must be provider/model, got {spec:?}"
                    )));
                }
                Some(ModelRef {
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                })
            }
            None => None,
        };

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ProcessError::Internal(format!("build opencode client: {e}")))?;

        Ok(Self {
            base_url,
            username: username.into(),
            password,
            agent: agent.into(),
            model,
            delete_sessions,
            client,
        })
    }

    /// Build an HTTP request with optional basic auth.
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self
            .client
            .request(method, format!("{}{}", self.base_url, path));
        match &self.password {
            Some(password) => builder.basic_auth(&self.username, Some(password)),
            None => builder,
        }
    }

    /// POST JSON and decode a JSON response.
    async fn post_json<B, R>(&self, path: &str, body: &B) -> Result<R, ProcessError>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let response = self
            .request(reqwest::Method::POST, path)
            .json(body)
            .send()
            .await
            .map_err(|e| ProcessError::Network(format!("opencode POST {path}: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(300).collect();
            return Err(ProcessError::Network(format!(
                "opencode POST {path} status {status}: {snippet}"
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| ProcessError::Network(format!("opencode POST {path} body: {e}")))
    }
}

#[async_trait]
impl AgentClient for OpenCodeClient {
    /// Create a session, send the prompt, and return the agent's text.
    async fn predict(
        &self,
        symbol: &str,
        candle_open: DateTime<Utc>,
    ) -> Result<String, ProcessError> {
        let close = crate::scheduler::candle_close(candle_open);
        let title = format!("signal-gateway {symbol} {}", close.to_rfc3339());
        let session: Session = self
            .post_json("/session", &NewSession { title: &title })
            .await?;

        let prompt = build_prompt(symbol, candle_open);
        let message = MessageRequest {
            agent: &self.agent,
            model: self.model.as_ref().map(|m| ModelRefBody {
                provider_id: &m.provider_id,
                model_id: &m.model_id,
            }),
            parts: vec![Part {
                kind: "text",
                text: prompt,
            }],
        };

        let path = format!("/session/{}/message", session.id);
        let result = self.post_json::<_, MessageResponse>(&path, &message).await;

        if self.delete_sessions
            && let Err(e) = self
                .request(reqwest::Method::DELETE, &format!("/session/{}", session.id))
                .send()
                .await
        {
            log::debug!("delete opencode session {} failed: {e}", session.id);
        }

        let response = result?;
        let text = response
            .parts
            .unwrap_or_default()
            .into_iter()
            .filter(|part| part.kind == "text")
            .filter_map(|part| part.text)
            .collect::<Vec<_>>()
            .join("\n");

        if text.trim().is_empty() {
            return Err(ProcessError::Internal(
                "agent returned no text parts".into(),
            ));
        }

        Ok(text)
    }
}

/// Build the user prompt sent to the trade-agent for one symbol and candle.
pub fn build_prompt(symbol: &str, candle_open: DateTime<Utc>) -> String {
    let close = crate::scheduler::candle_close(candle_open);
    format!(
        "Symbol: {symbol}\n\
         Trigger: 4h candle closed at {}.\n\
         Horizon: 24-72h.\n\
         Analyze recent news (last 72h) and compute RSI(21) on closed 4h candles yourself.\n\
         Return the standard Verdict plus the machine-readable JSON block.\n\
         Use NO-TRADE when evidence is insufficient or conflicting.\n",
        close.to_rfc3339()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn prompt_contains_symbol_and_close_time() {
        let open = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
        let prompt = build_prompt("BTC-USDT", open);
        assert!(prompt.contains("Symbol: BTC-USDT"));
        assert!(prompt.contains("2026-11-07T04:00:00+00:00"));
    }

    #[test]
    fn rejects_invalid_model_spec() {
        let result = OpenCodeClient::new(
            "http://127.0.0.1:4096",
            "opencode",
            None,
            "trade-agent",
            Some("no-slash".into()),
            true,
            Duration::from_secs(5),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_model_spec_with_extra_slash() {
        let result = OpenCodeClient::new(
            "http://127.0.0.1:4096",
            "opencode",
            None,
            "trade-agent",
            Some("opencode/gpt-5.5/extra".into()),
            true,
            Duration::from_secs(5),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_base_url() {
        let result = OpenCodeClient::new(
            "not a url",
            "opencode",
            None,
            "trade-agent",
            None,
            true,
            Duration::from_secs(5),
        );
        assert!(result.is_err());
    }
}

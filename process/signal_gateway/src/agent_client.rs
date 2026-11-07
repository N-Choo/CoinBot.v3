use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use share::ProcessError;

#[async_trait]
pub trait AgentClient: Send + Sync {
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

#[derive(Serialize)]
struct MessageRequest<'a> {
    agent: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
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
    parts: Vec<MessagePart>,
}

#[derive(Deserialize)]
struct MessagePart {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

pub struct OpenCodeClient {
    base_url: String,
    username: String,
    password: Option<String>,
    agent: String,
    model: Option<String>,
    delete_sessions: bool,
    client: reqwest::Client,
}

impl OpenCodeClient {
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
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ProcessError::Internal(format!("build opencode client: {e}")))?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            username: username.into(),
            password,
            agent: agent.into(),
            model,
            delete_sessions,
            client,
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self
            .client
            .request(method, format!("{}{}", self.base_url, path));
        match &self.password {
            Some(password) => builder.basic_auth(&self.username, Some(password)),
            None => builder,
        }
    }

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
            return Err(ProcessError::Network(format!(
                "opencode POST {path} status {}",
                response.status()
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
    async fn predict(
        &self,
        symbol: &str,
        candle_open: DateTime<Utc>,
    ) -> Result<String, ProcessError> {
        let title = format!("signal-gateway {symbol} {}", candle_open.to_rfc3339());
        let session: Session = self
            .post_json("/session", &NewSession { title: &title })
            .await?;

        let prompt = build_prompt(symbol, candle_open);
        let message = MessageRequest {
            agent: &self.agent,
            model: self.model.as_deref(),
            parts: vec![Part {
                kind: "text",
                text: prompt,
            }],
        };

        let response: MessageResponse = self
            .post_json(&format!("/session/{}/message", session.id), &message)
            .await?;

        if self.delete_sessions {
            let _ = self
                .request(reqwest::Method::DELETE, &format!("/session/{}", session.id))
                .send()
                .await;
        }

        let text = response
            .parts
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
}

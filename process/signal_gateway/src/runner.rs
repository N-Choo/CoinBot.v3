use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use share::{ProcessError, Signal as ProtoSignal};

use crate::agent_client::{self, AgentClient};
use crate::audit::{self, AuditRecord};
use crate::config::{Config, PriceCheck};
use crate::cursor;
use crate::mapper;
use crate::prediction::{self, Side};
use crate::price::LastPriceSource;
use crate::publisher::SignalPublisher;
use crate::state::{LastPublished, State};

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub agent_delays: Vec<Duration>,
    pub grpc_delays: Vec<Duration>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            agent_delays: vec![Duration::from_secs(5), Duration::from_secs(15)],
            grpc_delays: vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
            ],
        }
    }
}

pub struct Gateway {
    pub config: Config,
    pub state: State,
    pub agent: Arc<dyn AgentClient>,
    pub publisher: Arc<dyn SignalPublisher>,
    pub price: Arc<dyn LastPriceSource>,
    pub retry: RetryPolicy,
}

impl Gateway {
    pub fn new(
        config: Config,
        state: State,
        agent: Arc<dyn AgentClient>,
        publisher: Arc<dyn SignalPublisher>,
        price: Arc<dyn LastPriceSource>,
        retry: RetryPolicy,
    ) -> Self {
        Self {
            config,
            state,
            agent,
            publisher,
            price,
            retry,
        }
    }

    pub async fn run_cycle(
        &mut self,
        symbols: &[String],
        candle_open: DateTime<Utc>,
    ) -> Result<(), ProcessError> {
        let symbols = crate::symbols::normalize_symbols(symbols.to_vec());
        if symbols.is_empty() {
            log::warn!("no active symbols; skipping cycle");
            return Ok(());
        }

        let candle_open_ms = candle_open.timestamp_millis();
        let order = cursor::rotation(&symbols, self.state.cursor)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut processed = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;

        for symbol in &order {
            if self.state.is_processed(symbol, candle_open_ms) {
                skipped += 1;
                log::info!("skip {symbol}: candle already processed");
                continue;
            }

            match self.process_symbol(symbol, candle_open).await {
                Ok(()) => {
                    processed += 1;
                    self.state.mark_processed(symbol, candle_open_ms);
                    self.state.save(&self.config.state_path)?;
                }
                Err(e) => {
                    failed += 1;
                    log::error!("{symbol}: {e}; will retry next cycle");
                }
            }
        }

        self.state.cursor = cursor::advance(self.state.cursor, symbols.len());
        self.state.save(&self.config.state_path)?;
        log::info!(
            "cycle complete: processed={processed} skipped={skipped} failed={failed} cursor={}",
            self.state.cursor
        );
        Ok(())
    }

    async fn process_symbol(
        &mut self,
        symbol: &str,
        candle_open: DateTime<Utc>,
    ) -> Result<(), ProcessError> {
        let prompt = agent_client::build_prompt(symbol, candle_open);
        let text = match self.call_agent_with_retry(symbol, candle_open).await {
            Ok(text) => text,
            Err(e) => {
                self.write_audit(
                    symbol,
                    candle_open,
                    &prompt,
                    "",
                    "ERROR",
                    &format!("agent failed: {e}"),
                    None,
                    None,
                );
                return Err(e);
            }
        };

        let (verdict, validation, valid) = match prediction::extract_json(&text) {
            None => ("unknown".to_string(), "no JSON block".to_string(), None),
            Some(json) => match serde_json::from_str::<prediction::Prediction>(json) {
                Err(e) => ("unknown".to_string(), format!("invalid JSON: {e}"), None),
                Ok(parsed) => {
                    let verdict = match parsed.side {
                        Side::Long => "LONG",
                        Side::Short => "SHORT",
                        Side::NoTrade => "NO-TRADE",
                    }
                    .to_string();
                    match prediction::validate(&parsed, symbol, self.config.min_confidence) {
                        Ok(Some(valid)) => (verdict, "valid".to_string(), Some(valid)),
                        Ok(None) => (verdict, "no-trade".to_string(), None),
                        Err(e) => (verdict, format!("rejected: {e}"), None),
                    }
                }
            },
        };

        let Some(valid) = valid else {
            if verdict != "NO-TRADE" {
                let preview = prediction::sanitize_text(&text, 500);
                log::warn!("{symbol}: dropping prediction ({validation}): {preview}");
            }
            self.write_audit(
                symbol,
                candle_open,
                &prompt,
                &text,
                &verdict,
                &validation,
                None,
                None,
            );
            return Ok(());
        };

        if self.config.price_check == PriceCheck::Strict {
            let last = match self.price.last_price(symbol).await {
                Ok(last) => last,
                Err(e) => {
                    self.write_audit(
                        symbol,
                        candle_open,
                        &prompt,
                        &text,
                        &verdict,
                        &format!("price check failed: {e}"),
                        None,
                        None,
                    );
                    return Ok(());
                }
            };
            if !prediction::price_ok(valid.entry, last, self.config.max_entry_drift_pct) {
                let note = format!(
                    "rejected: entry {} is more than {}% from last {last}",
                    valid.entry, self.config.max_entry_drift_pct
                );
                self.write_audit(
                    symbol,
                    candle_open,
                    &prompt,
                    &text,
                    &verdict,
                    &note,
                    None,
                    None,
                );
                return Ok(());
            }
        }

        let seq = self.state.next_seq();
        let Some(signal) =
            mapper::to_proto(&valid, candle_open, Utc::now(), seq, self.config.signal_ttl)
        else {
            log::warn!("{symbol}: prediction could not be mapped to a signal");
            self.write_audit(
                symbol,
                candle_open,
                &prompt,
                &text,
                &verdict,
                "unmappable",
                None,
                None,
            );
            return Ok(());
        };

        if signal.expires_at_ms != 0 && signal.expires_at_ms < Utc::now().timestamp_millis() {
            log::warn!("{symbol}: {} expired before publish", signal.signal_id);
            self.write_audit(
                symbol,
                candle_open,
                &prompt,
                &text,
                &verdict,
                "expired before publish",
                Some(signal.signal_id.clone()),
                None,
            );
            return Ok(());
        }

        let ack = match self.publish_with_retry(signal.clone()).await {
            Ok(ack) => ack,
            Err(e) => {
                self.write_audit(
                    symbol,
                    candle_open,
                    &prompt,
                    &text,
                    &verdict,
                    &format!("publish failed: {e}"),
                    Some(signal.signal_id.clone()),
                    None,
                );
                return Err(e);
            }
        };
        let ack_note = format!("accepted={} reason={}", ack.accepted, ack.reason);

        self.state.last_published.insert(
            symbol.to_string(),
            LastPublished {
                candle_open_ms: candle_open.timestamp_millis(),
                signal_id: signal.signal_id.clone(),
            },
        );

        self.write_audit(
            symbol,
            candle_open,
            &prompt,
            &text,
            &verdict,
            "valid",
            Some(signal.signal_id.clone()),
            Some(ack_note),
        );

        if !ack.accepted && ack.reason != "duplicate" {
            log::warn!(
                "{symbol}: engine rejected {}: {}",
                signal.signal_id,
                ack.reason
            );
        }

        Ok(())
    }

    async fn call_agent_with_retry(
        &self,
        symbol: &str,
        candle_open: DateTime<Utc>,
    ) -> Result<String, ProcessError> {
        let attempts = self.retry.agent_delays.len() + 1;
        let mut last_error = None;

        for attempt in 0..attempts {
            match self.agent.predict(symbol, candle_open).await {
                Ok(text) => return Ok(text),
                Err(e) => {
                    let retryable = matches!(&e, ProcessError::Network(_));
                    last_error = Some(e);
                    if !retryable {
                        break;
                    }
                    if let Some(delay) = self.retry.agent_delays.get(attempt) {
                        tokio::time::sleep(*delay).await;
                    }
                }
            }
        }

        match last_error {
            Some(e) => Err(e),
            None => Err(ProcessError::Internal("no agent attempts".into())),
        }
    }

    async fn publish_with_retry(&self, signal: ProtoSignal) -> Result<share::Ack, ProcessError> {
        let attempts = self.retry.grpc_delays.len() + 1;
        let mut last_error = None;

        for attempt in 0..attempts {
            match self.publisher.publish(signal.clone()).await {
                Ok(ack) => return Ok(ack),
                Err(e) => {
                    let retryable = matches!(&e, ProcessError::Network(_));
                    last_error = Some(e);
                    if !retryable {
                        break;
                    }
                    if let Some(delay) = self.retry.grpc_delays.get(attempt) {
                        tokio::time::sleep(*delay).await;
                    }
                }
            }
        }

        match last_error {
            Some(e) => Err(e),
            None => Err(ProcessError::Internal("no publish attempts".into())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_audit(
        &self,
        symbol: &str,
        candle_open: DateTime<Utc>,
        prompt: &str,
        response: &str,
        verdict: &str,
        validation: &str,
        signal_id: Option<String>,
        ack: Option<String>,
    ) {
        if let Err(e) = audit::append(
            &self.config.audit_path,
            &AuditRecord {
                at: Utc::now().to_rfc3339(),
                candle_open: candle_open.to_rfc3339(),
                symbol: symbol.to_string(),
                prompt: prompt.to_string(),
                response: response.to_string(),
                verdict: verdict.to_string(),
                validation: validation.to_string(),
                signal_id,
                ack,
            },
        ) {
            log::warn!("{symbol}: audit write failed: {e}");
        }
    }
}

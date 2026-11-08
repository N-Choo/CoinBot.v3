# signal-gateway

Standalone Rust service that turns `trade-agent` predictions into
`trade-engine` signals.

- Runs 6 times per day, 90 seconds after each 4h UTC close
  (00/04/08/12/16/20).
- Pulls active symbols with `EngineService.GetActiveSymbols`.
- Calls the `trade-agent` opencode role once per symbol, sequentially, with a
  rotating round-robin start.
- Validates the strict JSON prediction, checks entry against the KuCoin last
  price, then publishes `EngineService.Publish`.
- Persists `seq`, the round-robin cursor, and processed candles in a local JSON
  file. Writes an audit JSONL per call.

## Run

```bash
cp process/signal_gateway/.env.example process/signal_gateway/.env
cargo run -p signal-gateway
```

Requires `opencode serve` (with the `trade-agent` role) and `trade-engine`
reachable at the configured endpoints.

## Environment

See `.env.example`. The important settings are `OPENCODE_BASE_URL`,
`OPENCODE_AGENT`, `OPENCODE_MODEL` (`provider/model`), `GRPC_TRADE_ENGINE_ENDPOINT`,
and `SIGNAL_GATEWAY_MIN_CONFIDENCE`.

## Behavior notes

- Signals expire at `emitted_at + SIGNAL_GATEWAY_SIGNAL_TTL_SECS` (default 4h),
  not at the candle close.
- In strict price-check mode, a failed KuCoin price fetch is treated as a
  rejection: nothing is published and the candle is marked processed.
- Audit writes are best-effort. A failed audit write is logged but does not
  prevent processing.
- If the process dies after a successful publish but before state is saved, the
  next run re-asks the agent for that candle. The deterministic `signal_id`
  deduplicates while the engine process is alive. A durable outbox is a
  follow-up.
- Ctrl+C and SIGTERM (Unix) finish the current cycle, then exit.

## State and audit

Default paths are `./state/signal_gateway.json` and `./state/audit.jsonl`.
Corrupt state refuses to start unless `SIGNAL_GATEWAY_RESET_STATE=true`.

## Test

```bash
cargo fmt -p signal-gateway -- --check
cargo clippy -p signal-gateway --all-targets -- -D warnings
cargo test -p signal-gateway
```

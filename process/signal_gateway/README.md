# signal-gateway

Standalone Rust service that turns `trade-agent` predictions into
`trade-engine` signals.

- Runs 6 times per day, 90 seconds after each 4h UTC close
  (00/04/08/12/16/20).
- Pulls active symbols with `EngineService.GetActiveSymbols`.
- Calls the `trade-agent` opencode role once per symbol, sequentially, with a
  rotating round-robin start.
- Validates the strict JSON prediction, checks entry against the KuCoin last
  price (when price check is enabled; default strict), then publishes
  `EngineService.Publish`.
- Persists `seq`, `last_published`, the round-robin cursor, and processed
  candles in a local JSON file. Writes an audit JSONL per call.

## Run

```bash
cd process/signal_gateway
cp .env.example .env
cargo run -p signal-gateway
```

`cargo run -p signal-gateway` from the repo root does not load the crate-local
`.env`; export the variables or use the `cd` form above. `make signal-gateway`
also uses the `cd` form.

Requires `opencode serve` (with the `trade-agent` role) and `trade-engine`
reachable at the configured endpoints.

## Docker

Build from the repository root (the workspace context is required):

```bash
docker build -f process/signal_gateway/Dockerfile -t signal-gateway .
```

The image does not contain a `.env` file. Inject configuration at runtime and
mount a volume for state and audit:

```bash
docker run --rm \
  --env-file process/signal_gateway/.env \
  -e OPENCODE_BASE_URL=http://host.docker.internal:4096 \
  -e GRPC_TRADE_ENGINE_ENDPOINT=http://host.docker.internal:50051 \
  -v signal-gateway-state:/app/state \
  signal-gateway
```

Replace the example endpoints with addresses reachable from the container.
The default state/audit paths are `/app/state/signal_gateway.json` and
`/app/state/audit.jsonl`. The service exposes no ports and runs as the
distroless `nonroot` user.

## Environment

See `.env.example`. The important settings are `OPENCODE_BASE_URL`,
`OPENCODE_AGENT`, `OPENCODE_MODEL` (`provider/model`), `GRPC_TRADE_ENGINE_ENDPOINT`,
`SIGNAL_GATEWAY_MIN_CONFIDENCE`, `SIGNAL_GATEWAY_PRICE_CHECK`,
`SIGNAL_GATEWAY_MAX_ENTRY_DRIFT_PCT`, `SIGNAL_GATEWAY_SIGNAL_TTL_SECS`, and
`SIGNAL_GATEWAY_RUN_ON_START`.

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
The audit file grows without rotation. Corrupt state refuses to start unless
`SIGNAL_GATEWAY_RESET_STATE=true`.

## Test

```bash
cargo fmt -p signal-gateway -- --check
cargo clippy -p signal-gateway --all-targets -- -D warnings
cargo test -p signal-gateway
```

FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS dev-builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json
COPY . .

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json --release
COPY . .
RUN cargo build -p signal-gateway --release && mkdir -p /app/state

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder --chown=nonroot:nonroot /app/target/release/signal-gateway /signal-gateway
COPY --from=builder --chown=nonroot:nonroot /app/state /app/state
ENV SIGNAL_GATEWAY_STATE_PATH=/app/state/signal_gateway.json
ENV SIGNAL_GATEWAY_AUDIT_PATH=/app/state/audit.jsonl
USER nonroot:nonroot
CMD ["/signal-gateway"]

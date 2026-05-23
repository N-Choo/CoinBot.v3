FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Dev stage (fast debug build) ──
FROM chef AS dev-builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json
COPY . .
RUN cargo build

# ── Prod stage (optimized release) ──
FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json
COPY . .
RUN cargo build --release

# ── Runtime ──
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /usr/local/bin/app
EXPOSE 8080
CMD ["app"]


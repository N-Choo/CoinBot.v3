FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS dev-builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook -p analyzer --features "live grpc" --recipe-path recipe.json
COPY . .

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook -p analyzer --features grpc --recipe-path recipe.json --release
COPY . .
RUN cargo build -p analyzer --features grpc --release --bin analyzer-serve

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/analyzer-serve /usr/local/bin/analyzer-serve
CMD ["analyzer-serve"]

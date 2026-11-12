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
RUN cargo build -p deposit --release

FROM alpine:3.20 AS health
ARG TARGETARCH
ARG HEALTH_PROBE_VERSION=v0.4.62
RUN wget -q -O "/grpc_health_probe-linux-${TARGETARCH}" \
      "https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${HEALTH_PROBE_VERSION}/grpc_health_probe-linux-${TARGETARCH}" && \
    wget -q -O /checksums.txt \
      "https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${HEALTH_PROBE_VERSION}/checksums.txt" && \
    cd / && grep " grpc_health_probe-linux-${TARGETARCH}$" checksums.txt | sha256sum -c - && \
    chmod +x "/grpc_health_probe-linux-${TARGETARCH}" && \
    mv "/grpc_health_probe-linux-${TARGETARCH}" /probe

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/deposit /
COPY --from=health /probe /usr/local/bin/grpc_health_probe
EXPOSE 50051
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
  CMD ["grpc_health_probe", "-addr=:50051"]
CMD ["/deposit"]

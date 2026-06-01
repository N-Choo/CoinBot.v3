FROM rust:1.85 AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p deposit

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/deposit /
EXPOSE 50051
CMD ["/deposit"]

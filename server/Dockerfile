FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# This step compiles your dependencies. 
# It will be much faster after the first successful run!
RUN cargo chef cook --recipe-path recipe.json

# Now build your actual application code
COPY . .
RUN cargo build --release

# Stage 3: The Runtime (The part that actually runs)
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /usr/local/bin/app

EXPOSE 8080
CMD ["app"]


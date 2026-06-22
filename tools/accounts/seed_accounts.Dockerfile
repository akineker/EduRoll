# STAGE 1: BUILDER
FROM rust:1.91-slim-bullseye AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/
COPY .sqlx/ /app/.sqlx/

# Use the sqlx offline cache
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin seed_accounts --workspace

# STAGE 2: RUNTIME
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libpq5 libssl1.1 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/seed_accounts /usr/local/bin/seed_accounts

CMD ["/usr/local/bin/seed_accounts"]

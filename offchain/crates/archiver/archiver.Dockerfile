#MULTI STAGE BUILD
# STAGE 1: BUILDER
FROM rust:1.91-slim-bullseye AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/
# sqlx query cache — required so the build can verify SQL without a live DB
COPY .sqlx/ /app/.sqlx/

# Build the archiver binary (offline mode uses the .sqlx cache)
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin archiver --workspace

# STAGE 2: RUNTIME
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libpq5 libssl1.1 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/archiver /usr/local/bin/archiver

CMD ["/usr/local/bin/archiver"]
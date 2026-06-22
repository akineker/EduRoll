#MULTI STAGE BUILD
# STAGE 1: BUILDER 
FROM rust:1.91-slim-bullseye AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*


WORKDIR /app
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/
COPY .sqlx/ /app/.sqlx/

ENV SQLX_OFFLINE=true
RUN cargo build --release --bin test_client --workspace

# STAGE 2: RUNTIME
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/test_client /usr/local/bin/test_client

CMD ["/usr/local/bin/test_client"]
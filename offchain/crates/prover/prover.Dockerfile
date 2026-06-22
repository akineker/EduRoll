#MULTI STAGE BUILD
# STAGE 1: BUILDER
FROM rust:1.91-slim-bullseye AS builder


RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

RUN npm install -g snarkjs@0.7.0 


WORKDIR /app
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/
COPY .sqlx/ /app/.sqlx/

ENV SQLX_OFFLINE=true
RUN cargo build --release --bin prover --workspace

# STAGE 2: RUNTIME
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates openssl libssl1.1 curl \
    && curl -fsSL https://deb.nodesource.com/setup_18.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g snarkjs@0.7.0 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/prover /usr/local/bin/prover

WORKDIR /app

CMD ["/usr/local/bin/prover"]
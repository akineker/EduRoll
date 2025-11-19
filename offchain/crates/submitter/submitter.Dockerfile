#MULTI STAGE BUILD
# --- STAGE 1: BUILDER ---
FROM rust:1.91-slim-bullseye AS builder

# Install PostgreSQL (libpq) and SSL dependencies
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/

# Build the submitter binary
RUN cargo build --release --bin submitter --workspace

# --- STAGE 2: RUNTIME ---
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libpq5 libssl1.1 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/submitter /usr/local/bin/submitter

CMD ["/usr/local/bin/submitter"]
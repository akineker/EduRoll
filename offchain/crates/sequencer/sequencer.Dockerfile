#MULTI STAGE BUILD
# --- STAGE 1: BUILDER ---
# COPY ALL PROJECT FILES FOR DOCKER
FROM rust:1.91-slim-bullseye AS builder

# Install PostgreSQL (libpq) and SSL dependencies required for sqlx
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# COPY the entire necessary workspace
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/

# Build the sequencer binary
RUN cargo build --release --bin sequencer --workspace

# --- STAGE 2: RUNTIME ---
FROM debian:bullseye-slim

# Install libpq and SSL libraries
RUN apt-get update && apt-get install -y libpq5 libssl1.1 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary
COPY --from=builder /app/target/release/sequencer /usr/local/bin/sequencer

# Set entrypoint
CMD ["/usr/local/bin/sequencer"]
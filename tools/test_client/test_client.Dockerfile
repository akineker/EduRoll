#MULTI STAGE BUILD
# --- STAGE 1: BUILDER ---
FROM rust:1.91-slim-bullseye AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# The test client only needs the Rust dependencies for signing/RPC
WORKDIR /app
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/

# Build the test_client binary
RUN cargo build --release --bin test_client --workspace

# --- STAGE 2: RUNTIME ---
FROM debian:bullseye-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary
COPY --from=builder /app/target/release/test_client /usr/local/bin/test_client

# The command is defined in docker-compose.yml to run the simulation
CMD ["/usr/local/bin/test_client"]
#MULTI STAGE BUILD
# --- STAGE 1: BUILDER ---
FROM rust:1.91-slim-bullseye AS builder

# Install ZK Prover Dependencies (for FFI/bindings)
# This includes C++ compiler, Node.js, and ZK-specific tools
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Install global ZK tools (e.g., for Circom witness generation or Noir artifacts)
# NOTE: Replace with specific Noir/Circom installation steps based on your chosen ZK framework setup
# For Circom/snarkjs:
RUN npm install -g snarkjs@0.7.0 

# Create workspace and copy source code
WORKDIR /app
# COPY the essential workspace files (Minimum Viable Copy for compilation)
COPY Cargo.toml /app/
COPY offchain/ /app/offchain/
COPY tools/ /app/tools/

# Build the specific prover crate and statically link
RUN cargo build --release --bin prover --workspace

# --- STAGE 2: RUNTIME ---
# Use a minimal base image for the final deployment
FROM debian:bullseye-slim
# Or use alpine for smallest size, but requires musl linking in builder stage (more complex)

# Install necessary runtime libraries (e.g., OpenSSL, ca-certificates)
RUN apt-get update && apt-get install -y ca-certificates openssl libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/prover /usr/local/bin/prover

# Create directory for ZK proving keys (mounted via docker-compose volume)
RUN mkdir -p /app/keys

# Run the Prover service
CMD ["/usr/local/bin/prover"]
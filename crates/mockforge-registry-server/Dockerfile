FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build registry server + the dedicated plugin scanner binary. The scanner
# carries wasmtime in a separate process so the registry server binary stays
# lean; the worker invokes it via subprocess (see
# crates/mockforge-registry-server/src/workers/plugin_scanner.rs).
RUN cargo build --release --package mockforge-registry-server && \
    cargo build --release \
        --package mockforge-plugin-loader \
        --features scanner-bin \
        --bin mockforge-plugin-scanner

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries. The scanner is optional — the worker falls back to
# in-process static analysis if it's missing — but installing it on PATH
# enables dynamic wasmtime instantiation + OS-level isolation.
COPY --from=builder /app/target/release/mockforge-registry-server /usr/local/bin/
COPY --from=builder /app/target/release/mockforge-plugin-scanner /usr/local/bin/

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run server
CMD ["mockforge-registry-server"]

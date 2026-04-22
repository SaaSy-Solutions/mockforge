# Multi-stage Docker build for MockForge
# Stage 0: Build the React Admin UI bundle so the Rust build can embed real assets
FROM node:22-slim AS ui-builder

WORKDIR /ui

# pnpm via Corepack — pnpm-lock.yaml is the source of truth for the UI workspace
RUN corepack enable

# Install deps first for layer caching. Skip the Playwright browser download —
# the image only needs Vite/TS build tooling, not the e2e runner.
ENV PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
COPY crates/mockforge-ui/ui/package.json crates/mockforge-ui/ui/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

# Copy UI source and build the production bundle (outputs to /ui/dist)
COPY crates/mockforge-ui/ui/ ./
RUN pnpm build

# Stage 1: Build the Rust application
# Use rust:1.90-slim (Trixie/testing-based) which has GLIBC 2.39+ required by native dependencies
FROM rust:1.90-slim AS builder

# Install required dependencies for building (including C++ for Kafka support)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    build-essential \
    g++ \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the workspace configuration files
COPY Cargo.toml Cargo.lock ./

# Remove test_openapi_demo, tests, and desktop-app from workspace members for Docker build
# Handle both quoted and unquoted formats, with/without trailing commas
RUN sed -i '/\s*"test_openapi_demo"\s*,\?\s*$/d' Cargo.toml && \
    sed -i '/\s*"tests"\s*,\?\s*$/d' Cargo.toml && \
    sed -i '/\s*"desktop-app"\s*,\?\s*$/d' Cargo.toml && \
    sed -i '/# Integration tests package/d' Cargo.toml

# Copy the crates directory
COPY crates/ ./crates/

# Copy any other necessary files
COPY examples/ ./examples/
COPY proto/ ./proto/
COPY config.example.yaml ./

# Drop the real UI bundle from the ui-builder stage into the expected path so
# build.rs embeds it via include_str!. Also remove build_ui.sh — build.rs
# invokes it during cargo build, but Node isn't installed here and dist/ is
# already built. build.rs only runs the script `if ui_build_script.exists()`.
COPY --from=ui-builder /ui/dist/ crates/mockforge-ui/ui/dist/
RUN rm -f crates/mockforge-ui/build_ui.sh

# Build the application in release mode with all protocol features enabled.
# Default features are HTTP-only; without `all-protocols` the ws/grpc/kafka/
# mqtt/amqp/smtp/tcp/graphql/ftp server spawns are cfg-gated out of the binary
# and the CLI silently runs as HTTP-only despite printing "configured" lines.
RUN cargo build --release --bin mockforge --features all-protocols

# Stage 2: Create the runtime image
# Use debian:trixie-slim to match builder's GLIBC version (2.39+)
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN groupadd -r mockforge && useradd -r -g mockforge mockforge

# Set the working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/mockforge /usr/local/bin/mockforge

# Copy example files and configuration
COPY --from=builder /app/examples/ ./examples/
COPY --from=builder /app/proto/ ./proto/
COPY --from=builder /app/config.example.yaml ./

# Create directories for fixtures and other data
RUN mkdir -p fixtures logs

# Change ownership to the non-root user
RUN chown -R mockforge:mockforge /app

# Switch to the non-root user
USER mockforge

# Expose ports (HTTP, WebSocket, gRPC, Admin UI)
EXPOSE 3000 3001 50051 9080

# Set default environment variables
ENV MOCKFORGE_LATENCY_ENABLED=true
ENV MOCKFORGE_FAILURES_ENABLED=false
ENV MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
# Mark that we're running in Docker (for Admin UI host detection)
ENV DOCKER_CONTAINER=true
# Default Admin UI to be accessible from outside container
ENV MOCKFORGE_ADMIN_HOST=0.0.0.0

# Default command
# Use full path to ensure binary is found regardless of PATH
CMD ["/usr/local/bin/mockforge", "serve", "--admin"]

# Multi-stage Docker build for MockForge
# Stage 1: Build the Rust application
FROM rust:1.90-slim AS builder

# Install required dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the workspace configuration files
COPY Cargo.toml Cargo.lock ./

# Remove test_openapi_demo from workspace members for Docker build
RUN sed -i '/"test_openapi_demo"/d' Cargo.toml

# Copy the crates directory
COPY crates/ ./crates/

# Copy any other necessary files
COPY examples/ ./examples/
COPY proto/ ./proto/
COPY config.example.yaml ./

# Build the application in release mode
RUN cargo build --release --package mockforge-cli

# Stage 2: Create the runtime image
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
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

# Default command
CMD ["mockforge", "serve", "--admin"]

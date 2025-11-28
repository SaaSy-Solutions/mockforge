#!/bin/bash

# Script to publish remaining MockForge crates for 0.2.8
set -e

CRATES=(
    "mockforge-recorder"
    "mockforge-plugin-registry"
    "mockforge-chaos"
    "mockforge-reporting"
    "mockforge-analytics"
    "mockforge-collab"
    "mockforge-plugin-loader"
    "mockforge-http"
    "mockforge-grpc"
    "mockforge-ws"
    "mockforge-graphql"
    "mockforge-mqtt"
    "mockforge-smtp"
    "mockforge-amqp"
    "mockforge-kafka"
    "mockforge-ftp"
    "mockforge-tcp"
    "mockforge-sdk"
    "mockforge-bench"
    "mockforge-test"
    "mockforge-tunnel"
    "mockforge-ui"
    "mockforge-cli"
)

for crate in "${CRATES[@]}"; do
    echo "Publishing $crate..."
    if cargo publish -p "$crate" --allow-dirty; then
        echo "✓ Successfully published $crate"
        echo "Waiting 30s for crates.io to process..."
        sleep 30
    else
        echo "✗ Failed to publish $crate"
        exit 1
    fi
done

echo "All crates published successfully!"

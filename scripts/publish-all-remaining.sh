#!/bin/bash

# Script to publish all remaining MockForge crates for 0.2.8
set -e

WORKSPACE_VERSION="0.2.8"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Function to convert dependencies for a crate
convert_deps() {
    local crate_name=$1
    local cargo_toml="crates/$crate_name/Cargo.toml"

    if [ ! -f "$cargo_toml" ]; then
        return 0
    fi

    print_status "Converting dependencies for $crate_name..."
    python3 - "$cargo_toml" "$WORKSPACE_VERSION" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
version = sys.argv[2]
text = path.read_text()
changed = False

targets = [
    ("mockforge-core", "../mockforge-core"),
    ("mockforge-data", "../mockforge-data"),
    ("mockforge-plugin-core", "../mockforge-plugin-core"),
    ("mockforge-plugin-sdk", "../mockforge-plugin-sdk"),
    ("mockforge-plugin-loader", "../mockforge-plugin-loader"),
    ("mockforge-plugin-registry", "../mockforge-plugin-registry"),
    ("mockforge-observability", "../mockforge-observability"),
    ("mockforge-tracing", "../mockforge-tracing"),
    ("mockforge-recorder", "../mockforge-recorder"),
    ("mockforge-reporting", "../mockforge-reporting"),
    ("mockforge-chaos", "../mockforge-chaos"),
    ("mockforge-analytics", "../mockforge-analytics"),
    ("mockforge-collab", "../mockforge-collab"),
    ("mockforge-http", "../mockforge-http"),
    ("mockforge-grpc", "../mockforge-grpc"),
    ("mockforge-ws", "../mockforge-ws"),
    ("mockforge-graphql", "../mockforge-graphql"),
    ("mockforge-mqtt", "../mockforge-mqtt"),
    ("mockforge-smtp", "../mockforge-smtp"),
    ("mockforge-amqp", "../mockforge-amqp"),
    ("mockforge-kafka", "../mockforge-kafka"),
    ("mockforge-ftp", "../mockforge-ftp"),
    ("mockforge-tcp", "../mockforge-tcp"),
    ("mockforge-sdk", "../mockforge-sdk"),
    ("mockforge-bench", "../mockforge-bench"),
    ("mockforge-test", "../mockforge-test"),
    ("mockforge-tunnel", "../mockforge-tunnel"),
    ("mockforge-ui", "../mockforge-ui"),
    ("mockforge-cli", "../mockforge-cli"),
    ("mockforge-scenarios", "../mockforge-scenarios"),
]

for name, rel in targets:
    pattern1 = rf'{name}\s*=\s*\{{\s*path\s*=\s*"{re.escape(rel)}"\s*\}}'
    pattern2 = rf'{name}\s*=\s*\{{\s*version\s*=\s*"[^"]*",\s*path\s*=\s*"{re.escape(rel)}"\s*\}}'
    pattern3 = rf'{name}\s*=\s*\{{\s*path\s*=\s*"{re.escape(rel)}",\s*version\s*=\s*"[^"]*"\s*\}}'

    replacement = f'{name} = "{version}"'

    for pattern in [pattern1, pattern2, pattern3]:
        new_text, count = re.subn(pattern, replacement, text)
        if count:
            text = new_text
            changed = True

if changed:
    path.write_text(text)
PY
}

# Function to publish a crate
publish_crate() {
    local crate_name=$1

    print_status "Publishing $crate_name..."
    convert_deps "$crate_name"

    # Check if already published
    if cargo search "$crate_name" --limit 1 2>/dev/null | grep -q "^$crate_name = \"$WORKSPACE_VERSION\""; then
        print_status "$crate_name already published, skipping..."
        return 0
    fi

    local publish_output=$(cargo publish -p "$crate_name" --allow-dirty 2>&1 | tee /tmp/publish_${crate_name}.log)
    local exit_code=${PIPESTATUS[0]}

    if [ $exit_code -eq 0 ]; then
        print_success "Published $crate_name"
        print_status "Waiting 30s for crates.io to process..."
        sleep 30
        return 0
    elif echo "$publish_output" | grep -q "already exists"; then
        print_status "$crate_name already published, skipping..."
        return 0
    else
        echo "Failed to publish $crate_name"
        return 1
    fi
}

# Publish in dependency order
print_status "Publishing remaining crates in dependency order..."

# Already published: mockforge-core, mockforge-plugin-core, mockforge-plugin-sdk,
# mockforge-observability, mockforge-tracing, mockforge-plugin-registry, mockforge-plugin-loader

# Publish crates with minimal dependencies first
publish_crate "mockforge-recorder"
publish_crate "mockforge-chaos"
publish_crate "mockforge-reporting"
publish_crate "mockforge-analytics"
publish_crate "mockforge-collab"

# Protocol crates
publish_crate "mockforge-smtp"
publish_crate "mockforge-mqtt"
publish_crate "mockforge-amqp"
publish_crate "mockforge-kafka"
publish_crate "mockforge-ftp"
publish_crate "mockforge-tcp"
publish_crate "mockforge-graphql"
publish_crate "mockforge-grpc"
publish_crate "mockforge-ws"
publish_crate "mockforge-http"

# Utility crates
publish_crate "mockforge-bench"
publish_crate "mockforge-test"
publish_crate "mockforge-sdk"
publish_crate "mockforge-tunnel"
publish_crate "mockforge-ui"
publish_crate "mockforge-cli"

print_success "All crates published successfully!"

#!/bin/bash

# Script to publish final remaining crates, handling indexing delays
set -e

WORKSPACE_VERSION="0.2.8"

# Convert dependencies function
convert_deps() {
    local crate_name=$1
    local cargo_toml="crates/$crate_name/Cargo.toml"

    if [ ! -f "$cargo_toml" ]; then
        return 0
    fi

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

# Publish with retries
publish_with_retry() {
    local crate_name=$1
    local max_retries=3
    local retry=0

    while [ $retry -lt $max_retries ]; do
        echo "Attempt $((retry + 1)): Publishing $crate_name..."
        convert_deps "$crate_name"

        if cargo publish -p "$crate_name" --no-verify --allow-dirty 2>&1 | tee /tmp/publish_${crate_name}.log | grep -q "Published\|Uploaded"; then
            echo "✓ Successfully published $crate_name"
            sleep 60  # Wait longer for indexing
            return 0
        elif grep -q "already exists" /tmp/publish_${crate_name}.log 2>/dev/null; then
            echo "✓ $crate_name already published"
            return 0
        else
            echo "✗ Failed, retrying in 60s..."
            retry=$((retry + 1))
            sleep 60
        fi
    done

    echo "✗ Failed to publish $crate_name after $max_retries attempts"
    return 1
}

# Publish in strict dependency order
echo "Publishing final crates in dependency order..."

# First: crates with minimal dependencies
publish_with_retry "mockforge-bench"
publish_with_retry "mockforge-http"
publish_with_retry "mockforge-test"

# Then: crates that depend on the above
publish_with_retry "mockforge-sdk"
publish_with_retry "mockforge-tunnel"
publish_with_retry "mockforge-ui"

# Finally: CLI depends on everything
publish_with_retry "mockforge-cli"

echo "Done!"

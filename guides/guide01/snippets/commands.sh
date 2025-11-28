#!/usr/bin/env bash
# Step 0: Install Mockforge (if not already installed)
# cargo install mockforge-cli

# Step 1: Create project and initialize
mkdir mf-hello && cd mf-hello
mockforge init --no-examples

# Step 2: Add route to mockforge.yaml (edit manually or use the config below)
# The config should have:
# routes:
#   - path: "/api/hello"
#     method: "GET"
#     response:
#       status: 200
#       body:
#         message: "Hello from Mockforge"

# Step 3: Start server
mockforge serve --http-port 4000 &
sleep 2  # Wait for server to start

# Step 4: Test the route
curl -s http://localhost:4000/api/hello | jq .

# Cleanup
pkill -f "mockforge serve"

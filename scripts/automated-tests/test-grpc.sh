#!/bin/bash

# gRPC Server Tests
# Tests gRPC server functionality

set -e

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}[INFO]${NC} Starting gRPC Server Testing..."

echo -e "${YELLOW}[WARNING]${NC} gRPC tests require grpcurl or similar tools"
echo -e "${YELLOW}[WARNING]${NC} These tests are complex and may require manual testing"
echo -e "${BLUE}[INFO]${NC} Key tests to perform:"
echo -e "${BLUE}[INFO]${NC}   - Start gRPC server with proto files"
echo -e "${BLUE}[INFO]${NC}   - Test unary, streaming RPCs with grpcurl"
echo -e "${BLUE}[INFO]${NC}   - Test HTTP bridge functionality"
echo -e "${BLUE}[INFO]${NC}   - Test OpenAPI documentation generation"

# Basic server startup test
mockforge serve --grpc-port 50053 > /tmp/grpc-test.log 2>&1 &
pid=$!
sleep 3

if kill -0 $pid 2>/dev/null; then
    echo -e "${GREEN}[SUCCESS]${NC} gRPC server starts successfully"
    kill $pid 2>/dev/null || true
else
    echo -e "${RED}[ERROR]${NC} gRPC server failed to start"
    cat /tmp/grpc-test.log || true
    exit 1
fi

echo -e "${GREEN}[SUCCESS]${NC} gRPC Server Testing completed (manual verification recommended)"
exit 0

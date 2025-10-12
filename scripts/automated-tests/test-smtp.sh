#!/bin/bash

# SMTP Email Testing
# Tests SMTP server functionality

set -e

BLUE='\033[0;34m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo -e "${BLUE}[INFO]${NC} Starting SMTP Email Testing..."

echo -e "${YELLOW}[WARNING]${NC} SMTP tests require SMTP client tools (telnet, swaks, etc.)"
echo -e "${YELLOW}[WARNING]${NC} These tests are complex and may require manual testing"

# Basic server startup test
mockforge serve --smtp --smtp-port 1025 > /tmp/smtp-test.log 2>&1 &
local pid=$!
sleep 3

if kill -0 $pid 2>/dev/null; then
    echo -e "${GREEN}[SUCCESS]${NC} SMTP server starts successfully"
    kill $pid 2>/dev/null || true
else
    echo -e "${RED}[ERROR]${NC} SMTP server failed to start"
    cat /tmp/smtp-test.log || true
    exit 1
fi

echo -e "${GREEN}[SUCCESS]${NC} SMTP Email Testing completed (manual verification recommended)"
exit 0

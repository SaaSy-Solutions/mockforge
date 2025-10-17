#!/bin/bash

# MockForge Development Script
# Starts both the Rust backend and UI dev server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
HTTP_PORT=${HTTP_PORT:-3000}
WS_PORT=${WS_PORT:-3002}
GRPC_PORT=${GRPC_PORT:-50051}
ADMIN_PORT=${ADMIN_PORT:-9080}
SPEC_FILE=${SPEC_FILE:-examples/openapi-demo.json}

echo -e "${BLUE}🚀 Starting MockForge Development Environment${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

# Determine available port inspection tool
PORT_CHECK_TOOL=""
if command -v lsof >/dev/null 2>&1; then
    PORT_CHECK_TOOL="lsof"
elif command -v ss >/dev/null 2>&1; then
    PORT_CHECK_TOOL="ss"
elif command -v netstat >/dev/null 2>&1; then
    PORT_CHECK_TOOL="netstat"
else
    echo -e "${YELLOW}⚠️  No port inspection tool (lsof/ss/netstat) detected; skipping port checks${NC}"
fi

# Determine whether to skip the UI dev server
UI_SKIP_VALUE=$(printf '%s' "${UI_SKIP:-false}" | tr '[:upper:]' '[:lower:]')
if [ "$UI_SKIP_VALUE" = "true" ] || [ "$UI_SKIP_VALUE" = "1" ]; then
    SKIP_UI=true
    echo -e "${YELLOW}⚠️  UI dev server will be skipped (UI_SKIP=${UI_SKIP})${NC}"
else
    SKIP_UI=false
fi

# Ensure npm is available before starting the UI when not skipped
if [ "$SKIP_UI" != "true" ]; then
    if ! command -v npm >/dev/null 2>&1; then
        echo -e "${RED}❌ npm is required to run the UI dev server${NC}"
        echo -e "${YELLOW}Install Node.js/npm or set UI_SKIP=true to skip the UI startup${NC}"
        exit 1
    fi
fi

# Function to check if a port is available
check_port() {
    local port=$1

    case "$PORT_CHECK_TOOL" in
        lsof)
            if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
                echo -e "${RED}❌ Port $port is already in use${NC}"
                return 1
            fi
            ;;
        ss)
            if ss -ltn | awk '{print $4}' | grep -q ":$port$"; then
                echo -e "${RED}❌ Port $port is already in use${NC}"
                return 1
            fi
            ;;
        netstat)
            if netstat -ltn | awk '{print $4}' | grep -q ":$port$"; then
                echo -e "${RED}❌ Port $port is already in use${NC}"
                return 1
            fi
            ;;
        "")
            # Port checks disabled; always succeed
            ;;
    esac

    return 0
}

# Check if required ports are available
echo -e "${YELLOW}Checking port availability...${NC}"
check_port $HTTP_PORT || exit 1
check_port $WS_PORT || exit 1
check_port $GRPC_PORT || exit 1
check_port $ADMIN_PORT || exit 1
echo -e "${GREEN}✅ All ports are available${NC}"
echo ""

# Function to start Rust backend
start_backend() {
    echo -e "${YELLOW}Starting Rust backend...${NC}"
    echo -e "${BLUE}📡 HTTP server on port $HTTP_PORT${NC}"
    echo -e "${BLUE}🔌 WebSocket server on port $WS_PORT${NC}"
    echo -e "${BLUE}⚡ gRPC server on port $GRPC_PORT${NC}"
    echo -e "${BLUE}🎛️ Admin API on port $ADMIN_PORT${NC}"
    echo ""

    RUST_LOG=debug cargo run -p mockforge-cli -- serve \
        --spec "$SPEC_FILE" \
        --http-port $HTTP_PORT \
        --ws-port $WS_PORT \
        --grpc-port $GRPC_PORT \
        --admin \
        --admin-port $ADMIN_PORT
}

# Function to wait for backend to be ready
wait_for_backend() {
    local max_attempts=90
    local attempt=1

    echo -e "${YELLOW}Waiting for backend to be ready...${NC}"

    while [ $attempt -le $max_attempts ]; do
        if curl -f -s "http://localhost:$ADMIN_PORT/__mockforge/health" >/dev/null 2>&1; then
            echo -e "${GREEN}✅ Backend is ready${NC}"
            return 0
        fi

        # Show progress every 5 attempts to reduce noise
        if [ $((attempt % 5)) -eq 0 ] || [ $attempt -le 3 ]; then
            echo -e "${BLUE}Waiting... (attempt $attempt/$max_attempts)${NC}"
        fi
        sleep 1
        ((attempt++))
    done

    echo -e "${RED}❌ Backend failed to start within ${max_attempts} seconds${NC}"
    return 1
}

# Function to start UI dev server
start_ui() {
    if [ "$SKIP_UI" = "true" ]; then
        echo -e "${YELLOW}UI dev server skipped by UI_SKIP flag${NC}"
        return 0
    fi
    echo -e "${YELLOW}Starting UI dev server...${NC}"
    echo -e "${BLUE}🌐 UI dev server starting (may take a moment)${NC}"
    echo ""

    cd crates/mockforge-ui/ui
    ADMIN_PORT=$ADMIN_PORT npm run dev
}

# Check if we should run in background mode
if [ "$1" = "--background" ] || [ "$1" = "-b" ]; then
    echo -e "${YELLOW}Starting services in background...${NC}"

    # Start backend in background
    start_backend &
    BACKEND_PID=$!

    # Wait a bit for backend to start
    sleep 3

    UI_PID=""
    if [ "$SKIP_UI" != "true" ]; then
        # Start UI in background
        start_ui &
        UI_PID=$!
    else
        echo -e "${YELLOW}UI dev server launch skipped in background mode${NC}"
    fi

    echo ""
    echo -e "${GREEN}✅ Services started in background${NC}"
    echo -e "${BLUE}📊 Backend PID: $BACKEND_PID${NC}"
    if [ -n "$UI_PID" ]; then
        echo -e "${BLUE}🎨 UI PID: $UI_PID${NC}"
    fi
    echo ""
    echo -e "${YELLOW}To stop services:${NC}"
    if [ -n "$UI_PID" ]; then
        echo -e "${BLUE}kill $BACKEND_PID $UI_PID${NC}"
    else
        echo -e "${BLUE}kill $BACKEND_PID${NC}"
    fi
    echo ""
    echo -e "${YELLOW}Access URLs:${NC}"
    echo -e "${BLUE}🌐 UI: http://localhost:5173${NC}"
    echo -e "${BLUE}🎛️ Admin: http://localhost:$ADMIN_PORT${NC}"

    # Wait for processes
    if [ -n "$UI_PID" ]; then
        wait $BACKEND_PID $UI_PID
    else
        wait $BACKEND_PID
    fi
else
    echo -e "${YELLOW}Starting services...${NC}"
    echo -e "${BLUE}Press Ctrl+C to stop both services${NC}"
    echo ""

    # Start both services in parallel using a trap to kill both on interrupt
    trap 'echo -e "\n${YELLOW}Stopping services...${NC}"; exit 0' INT

    # Start backend in background
    start_backend &
    BACKEND_PID=$!

    # Wait for backend to be ready
    if ! wait_for_backend; then
        echo -e "${RED}❌ Failed to start backend, stopping...${NC}"
        kill $BACKEND_PID 2>/dev/null || true
        exit 1
    fi

    UI_PID=""
    if [ "$SKIP_UI" != "true" ]; then
        # Start UI in foreground (this will be the primary process)
        start_ui &
        UI_PID=$!

        # Wait for either process to exit
        wait $BACKEND_PID $UI_PID
    else
        echo -e "${YELLOW}UI dev server launch skipped (UI_SKIP flag)${NC}"
        wait $BACKEND_PID
    fi
fi

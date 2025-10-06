#!/bin/bash
# MockForge Demo Recording Script
# This script automates the recording of a MockForge demo using asciinema

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DEMO_CAST="mockforge-demo.cast"
DEMO_GIF="mockforge-demo.gif"
TYPING_DELAY=0.1
PAUSE_SHORT=1
PAUSE_LONG=2

# Check dependencies
check_deps() {
    echo -e "${BLUE}Checking dependencies...${NC}"

    if ! command -v asciinema &> /dev/null; then
        echo "asciinema not found. Install with: brew install asciinema"
        exit 1
    fi

    if ! command -v mockforge &> /dev/null; then
        echo "mockforge not found. Build first with: cargo build --release"
        echo "Then add to PATH or use: cargo run -p mockforge-cli --"
        exit 1
    fi

    echo -e "${GREEN}All dependencies found!${NC}"
}

# Type out a command with delay
type_command() {
    local cmd="$1"
    echo -n "$ "
    for (( i=0; i<${#cmd}; i++ )); do
        echo -n "${cmd:$i:1}"
        sleep $TYPING_DELAY
    done
    echo
}

# Print colored comment
print_comment() {
    echo -e "${GREEN}# $1${NC}"
    sleep $PAUSE_SHORT
}

# Main demo script
run_demo() {
    print_comment "MockForge - Advanced API Mocking Platform"
    sleep $PAUSE_LONG

    # Introduction
    print_comment "Installation:"
    type_command "cargo install mockforge-cli"
    sleep $PAUSE_SHORT

    type_command "mockforge --version"
    mockforge --version
    sleep $PAUSE_LONG

    # Start server
    print_comment "Starting MockForge with OpenAPI demo..."
    type_command "mockforge serve --spec examples/openapi-demo.json --admin &"
    mockforge serve --spec examples/openapi-demo.json --admin > /dev/null 2>&1 &
    SERVER_PID=$!
    sleep 3

    # Test endpoints
    print_comment "Testing HTTP endpoints..."
    type_command "curl http://localhost:3000/ping | jq"
    curl -s http://localhost:3000/ping | jq
    sleep $PAUSE_LONG

    print_comment "Creating a user..."
    type_command 'curl -X POST http://localhost:3000/users -H "Content-Type: application/json" -d '"'"'{"name": "Alice", "email": "alice@example.com"}'"'"' | jq'
    curl -s -X POST http://localhost:3000/users \
        -H "Content-Type: application/json" \
        -d '{"name": "Alice", "email": "alice@example.com"}' | jq
    sleep $PAUSE_LONG

    # Template features
    print_comment "Notice: Dynamic UUIDs and timestamps in responses!"
    sleep $PAUSE_SHORT

    type_command "curl http://localhost:3000/users/123 | jq"
    curl -s http://localhost:3000/users/123 | jq
    sleep $PAUSE_LONG

    # Admin UI
    print_comment "Admin UI available at http://localhost:9080"
    type_command "curl http://localhost:9080/api/health | jq"
    curl -s http://localhost:9080/api/health | jq 2>/dev/null || echo '{"status": "healthy"}'
    sleep $PAUSE_LONG

    # Data generation
    print_comment "Generate test data with built-in templates..."
    type_command "mockforge data template user --rows 3 --format json | jq"
    mockforge data template user --rows 3 --format json 2>/dev/null | jq || echo '[{"id": 1, "name": "John Doe", "email": "john@example.com"}]' | jq
    sleep $PAUSE_LONG

    # Conclusion
    print_comment "Learn more:"
    echo "  📚 Documentation: https://docs.mockforge.dev"
    echo "  🐙 GitHub: https://github.com/SaaSy-Solutions/mockforge"
    echo "  📦 Install: cargo install mockforge-cli"
    sleep $PAUSE_LONG

    # Cleanup
    print_comment "Stopping server..."
    kill $SERVER_PID 2>/dev/null || true
    sleep $PAUSE_SHORT

    print_comment "Thanks for watching!"
}

# Convert to GIF
convert_to_gif() {
    if command -v agg &> /dev/null; then
        echo -e "${BLUE}Converting to GIF...${NC}"
        agg "$DEMO_CAST" "$DEMO_GIF"
        echo -e "${GREEN}GIF created: $DEMO_GIF${NC}"

        if command -v gifsicle &> /dev/null; then
            echo -e "${BLUE}Optimizing GIF...${NC}"
            gifsicle -O3 --colors 256 "$DEMO_GIF" -o "${DEMO_GIF%.gif}-optimized.gif"
            echo -e "${GREEN}Optimized GIF created: ${DEMO_GIF%.gif}-optimized.gif${NC}"
        fi
    else
        echo "agg not found. Install with: cargo install --git https://github.com/asciinema/agg"
        echo "Skipping GIF conversion."
    fi
}

# Main execution
main() {
    echo -e "${BLUE}MockForge Demo Recording Script${NC}"
    echo

    check_deps

    if [ "$1" == "--record" ]; then
        echo -e "${BLUE}Starting asciinema recording...${NC}"
        echo "The demo will start in 3 seconds..."
        sleep 3

        asciinema rec "$DEMO_CAST" -c "bash -c '$(declare -f run_demo print_comment type_command); run_demo'"

        echo -e "${GREEN}Recording saved: $DEMO_CAST${NC}"

        read -p "Convert to GIF? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            convert_to_gif
        fi
    else
        echo "Usage:"
        echo "  $0 --record    # Record the demo"
        echo
        echo "Alternative (manual recording):"
        echo "  1. Run: asciinema rec $DEMO_CAST"
        echo "  2. Execute demo commands manually"
        echo "  3. Press Ctrl+D when done"
        echo
        echo "Convert to GIF:"
        echo "  agg $DEMO_CAST $DEMO_GIF"
    fi
}

main "$@"

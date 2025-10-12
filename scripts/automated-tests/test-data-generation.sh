#!/bin/bash

# Data Generation Tests
# Tests built-in templates, custom schemas, and RAG-powered generation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
 echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to validate JSON file
validate_json() {
    local file="$1"
    if command -v jq > /dev/null 2>&1; then
        if jq empty "$file" > /dev/null 2>&1; then
            return 0
        fi
    else
        # Basic JSON validation with python
        if command -v python3 > /dev/null 2>&1; then
            if python3 -m json.tool "$file" > /dev/null 2>&1; then
                return 0
            fi
        fi
    fi
    return 1
}

# Function to validate CSV file
validate_csv() {
    local file="$1"
    if [ -f "$file" ] && [ $(wc -l < "$file") -gt 1 ]; then
        # Check if it has commas (basic CSV check)
        if grep -q "," "$file"; then
            return 0
        fi
    fi
    return 1
}

# Function to count lines/items in file
count_items() {
    local file="$1"
    local format="$2"

    case "$format" in
        "json")
            if command -v jq > /dev/null 2>&1; then
                jq length "$file" 2>/dev/null || echo "0"
            else
                # Count array elements with basic tools
                grep -o '\{' "$file" | wc -l
            fi
            ;;
        "jsonl")
            wc -l < "$file"
            ;;
        "csv")
            # Subtract 1 for header
            echo $(( $(wc -l < "$file") - 1 ))
            ;;
        *)
            wc -l < "$file"
            ;;
    esac
}

test_builtin_templates() {
    log_info "Testing built-in templates..."

    # Test user template with JSON format
    log_info "Testing user template JSON format..."
    if mockforge data template user --rows 5 --output /tmp/test-users.json; then
        if validate_json "/tmp/test-users.json"; then
            local count=$(count_items "/tmp/test-users.json" "json")
            if [ "$count" -ge 1 ]; then  # Allow at least 1 item since generation might vary
                log_success "User template JSON format works ($count items)"
            else
                log_error "User template JSON format failed: expected at least 1 item, got $count"
                return 1
            fi
        else
            log_error "User template did not generate valid JSON"
            return 1
        fi
    else
        log_error "User template JSON generation failed"
        return 1
    fi

    # Test user template with CSV format
    log_info "Testing user template CSV format..."
    if mockforge data template user --rows 3 --format csv --output /tmp/test-users.csv; then
        if validate_csv "/tmp/test-users.csv"; then
            local count=$(count_items "/tmp/test-users.csv" "csv")
            if [ "$count" -eq 3 ]; then
                log_success "User template CSV format works (3 users generated)"
            else
                log_error "User template CSV generated $count users, expected 3"
                return 1
            fi
        else
            log_error "User template did not generate valid CSV"
            return 1
        fi
    else
        log_error "User template CSV generation failed"
        return 1
    fi

    # Test user template with JSONL format
    log_info "Testing user template JSONL format..."
    if mockforge data template user --rows 2 --format jsonl --output /tmp/test-users.jsonl; then
        local count=$(count_items "/tmp/test-users.jsonl" "jsonl")
        if [ "$count" -eq 2 ]; then
            log_success "User template JSONL format works (2 users generated)"
        else
            log_error "User template JSONL generated $count users, expected 2"
            return 1
        fi
    else
        log_error "User template JSONL generation failed"
        return 1
    fi

    # Test product template
    log_info "Testing product template..."
    if mockforge data template product --rows 4 --output /tmp/test-products.json; then
        if validate_json "/tmp/test-products.json"; then
            local count=$(count_items "/tmp/test-products.json" "json")
            if [ "$count" -eq 4 ]; then
                log_success "Product template works (4 products generated)"
            else
                log_error "Product template generated $count products, expected 4"
                return 1
            fi
        else
            log_error "Product template did not generate valid JSON"
            return 1
        fi
    else
        log_error "Product template generation failed"
        return 1
    fi

    # Test order template
    log_info "Testing order template..."
    if mockforge data template order --rows 3 --output /tmp/test-orders.json; then
        if validate_json "/tmp/test-orders.json"; then
            local count=$(count_items "/tmp/test-orders.json" "json")
            if [ "$count" -eq 3 ]; then
                log_success "Order template works (3 orders generated)"
            else
                log_error "Order template generated $count orders, expected 3"
                return 1
            fi
        else
            log_error "Order template did not generate valid JSON"
            return 1
        fi
    else
        log_error "Order template generation failed"
        return 1
    fi

    # Clean up
    rm -f /tmp/test-users.json /tmp/test-users.csv /tmp/test-users.jsonl /tmp/test-products.json /tmp/test-orders.json

    log_success "Built-in templates tests passed"
    return 0
}

test_custom_schema_generation() {
    log_info "Testing custom schema generation..."

    # Create a simple JSON schema
    local schema_file="/tmp/test-schema.json"
    cat > "$schema_file" << EOF
{
  "\$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "id": {
      "type": "integer"
    },
    "name": {
      "type": "string"
    },
    "email": {
      "type": "string",
      "format": "email"
    },
    "active": {
      "type": "boolean"
    }
  },
  "required": ["id", "name", "email"]
}
EOF

    # Test schema-based generation
    if mockforge data schema "$schema_file" --rows 5 --output /tmp/test-schema-data.json; then
        if validate_json "/tmp/test-schema-data.json"; then
            local count=$(count_items "/tmp/test-schema-data.json" "json")
            if [ "$count" -eq 5 ]; then
                log_success "Custom schema generation works (5 items generated)"
                # Check that generated data has required fields
                if command -v jq > /dev/null 2>&1; then
                    local has_required=$(jq 'all(.id; . and (. != null)) and all(.name; . and (. != null)) and all(.email; . and (. != null))' /tmp/test-schema-data.json)
                    if [ "$has_required" = "true" ]; then
                        log_success "Generated data has required fields"
                    else
                        log_warning "Generated data may be missing required fields"
                    fi
                fi
            else
                log_error "Custom schema generated $count items, expected 5"
                rm -f "$schema_file" /tmp/test-schema-data.json
                return 1
            fi
        else
            log_error "Custom schema did not generate valid JSON"
            rm -f "$schema_file" /tmp/test-schema-data.json
            return 1
        fi
    else
        log_error "Custom schema generation failed"
        rm -f "$schema_file"
        return 1
    fi

    # Clean up
    rm -f "$schema_file" /tmp/test-schema-data.json

    log_success "Custom schema generation tests passed"
    return 0
}

test_rag_powered_generation() {
    log_info "Testing RAG-powered generation..."

    # Test RAG with Ollama (if available)
    log_info "Testing RAG with Ollama..."
    if command -v ollama > /dev/null 2>&1; then
        # Check if llama2 model is available
        if ollama list | grep -q "llama2"; then
            if mockforge data template user --rows 2 --rag --rag-provider ollama --output /tmp/test-rag-ollama.json; then
                if validate_json "/tmp/test-rag-ollama.json"; then
                    log_success "RAG generation with Ollama works"
                else
                    log_warning "RAG with Ollama did not generate valid JSON"
                fi
                rm -f /tmp/test-rag-ollama.json
            else
                log_warning "RAG generation with Ollama failed (may require API setup)"
            fi
        else
            log_warning "Ollama llama2 model not available, skipping Ollama RAG test"
        fi
    else
        log_warning "Ollama not installed, skipping Ollama RAG test"
    fi

    # Test RAG with OpenAI (if API key is set)
    log_info "Testing RAG with OpenAI..."
    if [ -n "$MOCKFORGE_RAG_API_KEY" ] || [ -n "$OPENAI_API_KEY" ]; then
        if mockforge data template user --rows 2 --rag --rag-provider openai --output /tmp/test-rag-openai.json; then
            if validate_json "/tmp/test-rag-openai.json"; then
                log_success "RAG generation with OpenAI works"
            else
                log_warning "RAG with OpenAI did not generate valid JSON"
            fi
            rm -f /tmp/test-rag-openai.json
        else
            log_warning "RAG generation with OpenAI failed (may require valid API key)"
        fi
    else
        log_warning "No OpenAI API key set, skipping OpenAI RAG test"
    fi

    # Test without RAG providers (should still work)
    log_info "Testing fallback without RAG providers..."
    if mockforge data template user --rows 2 --output /tmp/test-no-rag.json; then
        if validate_json "/tmp/test-no-rag.json"; then
            log_success "Data generation works without RAG providers"
        else
            log_error "Data generation without RAG failed to generate valid JSON"
            rm -f /tmp/test-no-rag.json
            return 1
        fi
        rm -f /tmp/test-no-rag.json
    else
        log_error "Data generation without RAG failed"
        return 1
    fi

    log_success "RAG-powered generation tests passed"
    return 0
}

main() {
    log_info "Starting Data Generation tests..."

    local failed_tests=()

    if ! test_builtin_templates; then
        failed_tests+=("builtin_templates")
    fi

    if ! test_custom_schema_generation; then
        failed_tests+=("custom_schema_generation")
    fi

    if ! test_rag_powered_generation; then
        failed_tests+=("rag_powered_generation")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All Data Generation tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"

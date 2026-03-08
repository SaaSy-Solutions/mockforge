#!/bin/bash

set -euo pipefail

BLUE='\033[0;34m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

section_name=$(basename "$0" .sh | sed 's/test-//' | sed 's/-/ /g' | sed 's/\b\w/\U&/g')

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
BOOK_DIR="$REPO_ROOT/book"
BOOK_SRC_DIR="$BOOK_DIR/src"
SUMMARY_FILE="$BOOK_SRC_DIR/SUMMARY.md"

log_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1" >&2
}

require_command() {
  local cmd="$1"
  local install_hint="$2"

  if ! command -v "$cmd" >/dev/null 2>&1; then
    log_error "Missing required command: $cmd"
    log_error "$install_hint"
    exit 1
  fi
}

check_summary_targets() {
  log_info "Validating SUMMARY.md targets..."

  local failures=0
  local current_dir="$BOOK_SRC_DIR"

  while IFS= read -r line; do
    local trimmed
    trimmed=$(printf '%s' "$line" | sed 's/^[[:space:]]*//')

    if [[ "$trimmed" =~ ^# ]]; then
      continue
    fi

    local target
    target=$(printf '%s\n' "$line" | perl -ne 'if (/\[[^\]]+\]\(([^)#]+)(?:#[^)]+)?\)/) { print $1; }')

    if [[ -z "$target" ]]; then
      continue
    fi

    if [[ "$target" =~ ^https?:// ]]; then
      continue
    fi

    local resolved
    resolved=$(cd "$current_dir" && python3 - <<'PY' "$target"
from pathlib import Path
import sys
print((Path.cwd() / sys.argv[1]).resolve())
PY
)

    if [[ ! -f "$resolved" ]]; then
      log_error "Missing SUMMARY target: $target"
      failures=$((failures + 1))
    fi
  done < "$SUMMARY_FILE"

  if [[ "$failures" -gt 0 ]]; then
    log_error "SUMMARY.md validation failed with $failures missing target(s)"
    exit 1
  fi

  log_success "SUMMARY.md targets look valid"
}

build_book() {
  log_info "Building mdBook documentation..."

  (
    cd "$BOOK_DIR"
    PATH="$HOME/.cargo/bin:$PATH" mdbook build
  )

  log_success "mdBook build completed successfully"
}

main() {
  log_info "Starting $section_name Testing..."

  if [[ ! -d "$BOOK_DIR" ]]; then
    log_error "Book directory not found: $BOOK_DIR"
    exit 1
  fi

  if [[ ! -f "$SUMMARY_FILE" ]]; then
    log_error "SUMMARY.md not found: $SUMMARY_FILE"
    exit 1
  fi

  require_command "python3" "Install Python 3 so path validation can resolve mdBook links."
  require_command "mdbook" "Install mdBook with 'cargo install mdbook'."
  require_command "mdbook-toc" "Install the required preprocessor with 'cargo install mdbook-toc'."

  if grep -q 'command = "mdbook-mermaid"' "$BOOK_DIR/book.toml"; then
    require_command "mdbook-mermaid" "Install the optional preprocessor with 'cargo install mdbook-mermaid'."
  else
    log_warn "mdbook-mermaid is not enabled in book.toml; skipping mermaid preprocessor check"
  fi

  check_summary_targets
  build_book

  log_success "$section_name Testing completed"
}

main "$@"

#!/bin/bash

# MockForge Crates Publishing Script
# This script publishes crates to crates.io in the correct dependency order

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DRY_RUN=${DRY_RUN:-false}
WAIT_TIME=${WAIT_TIME:-30}  # Seconds to wait between publishes
CRATES_IO_TOKEN=${CRATES_IO_TOKEN:-""}

# Determine current workspace version
WORKSPACE_VERSION=$(
    python3 - <<'PY'
import tomllib
from pathlib import Path

data = tomllib.loads(Path("Cargo.toml").read_text())
print(data["workspace"]["package"]["version"])
PY
)

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if crates.io token is set
check_token() {
    if [ -z "$CRATES_IO_TOKEN" ] && [ -z "$CARGO_REGISTRY_TOKEN" ]; then
        print_error "CRATES_IO_TOKEN or CARGO_REGISTRY_TOKEN environment variable is not set!"
        print_status "Please set it with: export CRATES_IO_TOKEN=your_token_here"
        print_status "Or use: export CARGO_REGISTRY_TOKEN=your_token_here"
        print_status "Get your token from: https://crates.io/me"
        exit 1
    fi
    # Use CARGO_REGISTRY_TOKEN if CRATES_IO_TOKEN is not set
    if [ -z "$CRATES_IO_TOKEN" ] && [ -n "$CARGO_REGISTRY_TOKEN" ]; then
        export CRATES_IO_TOKEN="$CARGO_REGISTRY_TOKEN"
    fi
}

# Function to wait for crates.io to process
wait_for_processing() {
    if [ "$DRY_RUN" = "false" ]; then
        print_status "Waiting ${WAIT_TIME}s for crates.io to process..."
        sleep $WAIT_TIME
    fi
}

# Function to check if a crate already exists on crates.io
crate_exists() {
    local crate_name=$1

    if cargo search "$crate_name" --limit 1 | grep -q "^$crate_name = \"$WORKSPACE_VERSION\""; then
        return 0  # Target version already exists
    fi
    return 1
}

# Function to handle publish errors
handle_publish_error() {
    local crate_name=$1
    local dry_run_flag=$2

    # Check if the error is because the crate already exists
    if [ "$DRY_RUN" = "false" ] && cargo publish -p "$crate_name" --dry-run --allow-dirty 2>&1 | grep -q "already exists"; then
        print_warning "$crate_name already exists on crates.io, skipping..."
        return 0
    else
        print_error "Failed to publish $crate_name"
        print_status "This might be due to authentication. Make sure you have:"
        print_status "1. Run 'cargo login' with your token, OR"
        print_status "2. Set CRATES_IO_TOKEN environment variable"
        exit 1
    fi
}

# Function to check if crate directory exists
crate_dir_exists() {
    local crate_name=$1
    if [ -d "crates/$crate_name" ] && [ -f "crates/$crate_name/Cargo.toml" ]; then
        return 0
    fi
    return 1
}

# Function to check if crate is in workspace
crate_in_workspace() {
    local crate_name=$1
    # Check if we can locate the crate using cargo locate-project
    # This is more reliable than grepping metadata
    if cargo locate-project --manifest-path "crates/$crate_name/Cargo.toml" &>/dev/null; then
        return 0
    fi
    # Fallback: check if crate exists in workspace metadata
    if cargo metadata --format-version 1 --no-deps 2>/dev/null | \
       python3 -c "import sys, json; data = json.load(sys.stdin); packages = [p['name'] for p in data.get('packages', [])]; sys.exit(0 if '$crate_name' in packages else 1)" 2>/dev/null; then
        return 0
    fi
    return 1
}

# Function to publish a crate
publish_crate() {
    local crate_name=$1
    local dry_run_flag=""

    # Check if crate directory exists and is in workspace
    if ! crate_dir_exists "$crate_name"; then
        print_warning "$crate_name directory not found, skipping..."
        return 0
    fi

    if ! crate_in_workspace "$crate_name"; then
        print_warning "$crate_name is not in workspace, skipping..."
        return 0
    fi

    if [ "$DRY_RUN" = "true" ]; then
        dry_run_flag="--dry-run"
        print_status "DRY RUN: Would publish $crate_name"
    else
        # Check if crate already exists on crates.io
        if crate_exists "$crate_name"; then
            print_warning "$crate_name already exists on crates.io, skipping..."
            return 0
        fi

        print_status "Publishing $crate_name..."
    fi

    # Set token for cargo publish if available
    local publish_env=""
    if [ -n "$CRATES_IO_TOKEN" ]; then
        publish_env="CARGO_REGISTRY_TOKEN=$CRATES_IO_TOKEN"
    elif [ -n "$CARGO_REGISTRY_TOKEN" ]; then
        publish_env="CARGO_REGISTRY_TOKEN=$CARGO_REGISTRY_TOKEN"
    fi

    # Use --no-verify to skip verification, which can fail when workspace crates
    # depend on unpublished versions of other workspace crates
    local no_verify_flag="--no-verify"
    if [ "$DRY_RUN" = "true" ]; then
        no_verify_flag=""  # Don't use --no-verify for dry runs, we want to see verification errors
    fi

    # Temporarily remove dependent crates from workspace to avoid dependency resolution issues
    # This is needed because Cargo resolves workspace dependencies even with --no-verify
    local temp_workspace_modified=false
    local removed_crates=""
    if [ "$DRY_RUN" = "false" ]; then
        # Find crates that depend on this crate and temporarily remove them from workspace
        # We need to do this before cargo publish to avoid dependency resolution errors
        local dependent_crates=$(cargo metadata --format-version 1 --no-deps 2>/dev/null | \
            python3 -c "
import sys, json
data = json.load(sys.stdin)
target_name = '$crate_name'
dependents = []
for pkg in data.get('packages', []):
    pkg_name = pkg.get('name', '')
    if pkg_name == target_name:
        continue
    for dep in pkg.get('dependencies', []):
        if dep.get('name') == target_name:
            # Extract crate directory name from manifest path
            manifest = pkg.get('manifest_path', '')
            if 'crates/' in manifest:
                crate_dir = manifest.split('crates/')[1].split('/')[0]
                dependents.append(crate_dir)
            break
print(' '.join(set(dependents)))
" 2>/dev/null || echo "")

        if [ -n "$dependent_crates" ]; then
            for dep_crate in $dependent_crates; do
                # Remove from workspace temporarily
                if grep -q "\"crates/$dep_crate\"," Cargo.toml; then
                    sed -i "/\"crates\/$dep_crate\",/d" Cargo.toml
                    removed_crates="$removed_crates $dep_crate"
                    temp_workspace_modified=true
                fi
            done
        fi
    fi

    if [ -n "$publish_env" ]; then
        if env $publish_env cargo publish -p "$crate_name" $dry_run_flag $no_verify_flag --allow-dirty; then
            print_success "Successfully published $crate_name"
        else
            # Check if it's a "package not found" error
            if env $publish_env cargo publish -p "$crate_name" --dry-run --allow-dirty 2>&1 | grep -q "package ID specification.*did not match"; then
                print_warning "$crate_name not found in workspace or not publishable, skipping..."
                return 0
            else
                handle_publish_error "$crate_name" "$dry_run_flag"
            fi
        fi
    else
        if cargo publish -p "$crate_name" $dry_run_flag $no_verify_flag --allow-dirty; then
            print_success "Successfully published $crate_name"
        else
            # Check if it's a "package not found" error
            if cargo publish -p "$crate_name" --dry-run --allow-dirty 2>&1 | grep -q "package ID specification.*did not match"; then
                print_warning "$crate_name not found in workspace or not publishable, skipping..."
                return 0
            else
                handle_publish_error "$crate_name" "$dry_run_flag"
            fi
        fi
    fi

    # Restore dependent crates' dependencies if we modified them
    if [ "$temp_deps_modified" = "true" ]; then
        for dep_crate_dir in $modified_crates; do
            local dep_cargo_toml="crates/$dep_crate_dir/Cargo.toml"
            if [ -f "$dep_cargo_toml" ]; then
                # Convert back to version dependency (remove path)
                # Handle both table form and short form
                if grep -q "$crate_name = { version = \"$WORKSPACE_VERSION\", path = \"../$crate_name\" }" "$dep_cargo_toml"; then
                    # Was short form, convert back to short form
                    sed -i "s|$crate_name = { version = \"$WORKSPACE_VERSION\", path = \"../$crate_name\" }|$crate_name = \"$WORKSPACE_VERSION\"|g" "$dep_cargo_toml"
                else
                    # Was table form, just remove path
                    sed -i "s|, path = \"../$crate_name\"||g" "$dep_cargo_toml"
                fi
            fi
        done
        if [ -n "$modified_crates" ]; then
            print_status "Restored dependencies in: $modified_crates"
        fi
    fi
}

# Function to check if a crate version is already published on crates.io
crate_version_published() {
    local crate_name=$1
    local version=$2
    if cargo search "$crate_name" --limit 1 2>/dev/null | grep -q "^$crate_name = \"$version\""; then
        return 0
    fi
    return 1
}

# Function to convert dependencies for a specific crate
convert_crate_dependencies() {
    local crate_name=$1

    # Check if crate directory exists and is in workspace
    if ! crate_dir_exists "$crate_name"; then
        print_warning "$crate_name directory not found, skipping dependency conversion..."
        return 0
    fi

    if ! crate_in_workspace "$crate_name"; then
        print_warning "$crate_name is not in workspace, skipping dependency conversion..."
        return 0
    fi

    local cargo_toml="crates/$crate_name/Cargo.toml"

    if [ -f "$cargo_toml" ]; then
        print_status "Converting dependencies for $crate_name..."
        # Build list of crates that will be published in this batch
        # For Phase 1, include all Phase 1 crates; for Phase 2, include all Phase 1 + Phase 2 crates
        local published_crates=""
        local phase1_crates="mockforge-template-expansion mockforge-core mockforge-data mockforge-plugin-core mockforge-observability mockforge-tracing mockforge-plugin-sdk mockforge-recorder mockforge-plugin-registry mockforge-chaos mockforge-reporting mockforge-analytics mockforge-pipelines mockforge-collab"
        local phase2_crates="mockforge-performance mockforge-route-chaos mockforge-plugin-loader mockforge-schema mockforge-mqtt mockforge-scenarios mockforge-smtp mockforge-ws mockforge-http mockforge-grpc mockforge-graphql mockforge-amqp mockforge-kafka mockforge-ftp mockforge-tcp mockforge-sdk mockforge-bench mockforge-test mockforge-vbr mockforge-tunnel mockforge-ui mockforge-cli"

        # Check which phase we're in based on the crate being published
        local all_crates="$phase1_crates $phase2_crates"
        local current_phase=""
        if [[ " $phase1_crates " =~ " $crate_name " ]]; then
            current_phase="phase1"
        elif [[ " $phase2_crates " =~ " $crate_name " ]]; then
            current_phase="phase2"
        fi

        for dep_crate in $all_crates; do
            # Only include if already published on crates.io
            # This prevents converting dependencies to versions that don't exist yet
            if crate_version_published "$dep_crate" "$WORKSPACE_VERSION"; then
                published_crates="$published_crates $dep_crate"
            fi
        done

        python3 - "$cargo_toml" "$WORKSPACE_VERSION" "$published_crates" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
version = sys.argv[2]
published = set(sys.argv[3].split()) if len(sys.argv) > 3 and sys.argv[3] else set()
text = path.read_text()
changed = False

# List of all internal mockforge crates that might be dependencies
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
    ("mockforge-vbr", "../mockforge-vbr"),
    ("mockforge-tunnel", "../mockforge-tunnel"),
    ("mockforge-ui", "../mockforge-ui"),
    ("mockforge-cli", "../mockforge-cli"),
    ("mockforge-scenarios", "../mockforge-scenarios"),
    ("mockforge-schema", "../mockforge-schema"),
    ("mockforge-template-expansion", "../mockforge-template-expansion"),
    ("mockforge-route-chaos", "../mockforge-route-chaos"),
]

for name, rel in targets:
    # Only convert if this crate has been published
    if name not in published:
        continue

    # Match dependency block with path - extract features and optional flags
    # Pattern matches: { path = "...", features = [...], optional = true } or variations
    pattern = rf'{name}\s*=\s*\{{([^}}]*path\s*=\s*"{re.escape(rel)}"[^}}]*)\}}'

    def replace_dep(match):
        dep_content = match.group(1)
        # Extract features using regex (handles simple arrays)
        features_match = re.search(r'features\s*=\s*(\[[^\]]*\])', dep_content)
        features = features_match.group(0) if features_match else None  # Get "features = [...]"

        # Check if optional
        is_optional = re.search(r'optional\s*=\s*true', dep_content) is not None

        # Build replacement - preserve features and optional flag
        parts = [f'version = "{version}"']
        if is_optional:
            parts.append('optional = true')
        if features:
            parts.append(features)

        if len(parts) > 1:
            return f'{name} = {{ {", ".join(parts)} }}'
        else:
            return f'{name} = "{version}"'

    new_text = re.sub(pattern, replace_dep, text)
    if new_text != text:
        text = new_text
        changed = True

publish_pattern = re.compile(r'(publish\s*=\s*)false(\s*#.*)?')
new_text, count = publish_pattern.subn(lambda m: f"{m.group(1)}true{m.group(2) or ''}", text)
if count:
    text = new_text
    changed = True

if changed:
    path.write_text(text)
PY
    fi
}

# Function to convert path dependencies to version dependencies (legacy - converts all at once)
convert_dependencies() {
    print_status "Converting path dependencies to version dependencies..."

    # List of crates that need dependency conversion
    local crates_to_convert=(
        "mockforge-data"
        "mockforge-observability"
        "mockforge-tracing"
        "mockforge-recorder"
        "mockforge-plugin-registry"
        "mockforge-reporting"
        "mockforge-chaos"
        "mockforge-analytics"
        "mockforge-collab"
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
        "mockforge-plugin-loader"
        "mockforge-k8s-operator"
        "mockforge-registry-server"
        "mockforge-ui"
        "mockforge-tunnel"
        "mockforge-cli"
        "mockforge-schema"
    )

    for crate in "${crates_to_convert[@]}"; do
        convert_crate_dependencies "$crate"
    done

    print_success "Dependency conversion completed"
}

# Function to restore path dependencies (for development)
restore_dependencies() {
    print_status "Restoring path dependencies for development..."

    local crates_to_restore=(
        "mockforge-core"
        "mockforge-data"
        "mockforge-template-expansion"
        "mockforge-plugin-core"
        "mockforge-plugin-sdk"
        "mockforge-observability"
        "mockforge-tracing"
        "mockforge-recorder"
        "mockforge-plugin-registry"
        "mockforge-reporting"
        "mockforge-chaos"
        "mockforge-analytics"
        "mockforge-collab"
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
        "mockforge-plugin-loader"
        "mockforge-k8s-operator"
        "mockforge-registry-server"
        "mockforge-ui"
        "mockforge-tunnel"
        "mockforge-cli"
        "mockforge-schema"
        "mockforge-pipelines"
        "mockforge-route-chaos"
        "mockforge-scenarios"
        "mockforge-world-state"
        "mockforge-vbr"
        "mockforge-performance"
    )

    for crate in "${crates_to_restore[@]}"; do
        local cargo_toml="crates/$crate/Cargo.toml"
        if [ -f "$cargo_toml" ]; then
            python3 - "$cargo_toml" "$WORKSPACE_VERSION" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
version = sys.argv[2]
text = path.read_text()
changed = False

targets = [
    # Note: mockforge-cli publishes as "mockforge-cli", not "mockforge"
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
    ("mockforge-vbr", "../mockforge-vbr"),
    ("mockforge-tunnel", "../mockforge-tunnel"),
    ("mockforge-ui", "../mockforge-ui"),
    ("mockforge-cli", "../mockforge-cli"),
    ("mockforge-scenarios", "../mockforge-scenarios"),
    ("mockforge-schema", "../mockforge-schema"),
    ("mockforge-template-expansion", "../mockforge-template-expansion"),
    ("mockforge-route-chaos", "../mockforge-route-chaos"),
    ("mockforge-performance", "../mockforge-performance"),
    ("mockforge-world-state", "../mockforge-world-state"),
]

for name, rel in targets:
    # Match both simple form: name = "version"
    # and table form: name = { version = "version", optional = true, ... }
    # First, handle table form (with version but no path)
    # Match table form that has version but doesn't have path
    # Use a more flexible pattern that handles nested braces
    table_pattern = rf'{name}\s*=\s*\{{([^}}]*version\s*=\s*"{re.escape(version)}"[^}}]*)\}}'
    def replace_table(match):
        full_match = match.group(0)
        dep_content = match.group(1)
        # Only replace if path is not already present
        if 'path =' in dep_content or 'path=' in dep_content:
            return full_match  # Already has path, don't change

        # Extract optional flag and features if present
        is_optional = re.search(r'optional\s*=\s*true', dep_content) is not None
        features_match = re.search(r'features\s*=\s*(\[[^\]]*\])', dep_content)
        features = features_match.group(0) if features_match else None

        # Build replacement with path
        parts = [f'version = "{version}"', f'path = "{rel}"']
        if is_optional:
            parts.append('optional = true')
        if features:
            parts.append(features)

        return f'{name} = {{ {", ".join(parts)} }}'

    # Try the pattern match
    new_text = re.sub(table_pattern, replace_table, text)
    if new_text != text:
        text = new_text
        changed = True
    else:
        # Fallback: try a simpler pattern that matches any table form with this version
        # This handles cases where the pattern might not match due to formatting
        simple_table_pattern = rf'{name}\s*=\s*\{{[^}}]*version\s*=\s*"{re.escape(version)}"[^}}]*\}}'
        if re.search(simple_table_pattern, text) and 'path =' not in text[text.find(f'{name} ='):text.find(f'{name} =')+200] if f'{name} =' in text else False:
            # Manually find and replace
            def manual_replace(m):
                full = m.group(0)
                if 'path =' in full or 'path=' in full:
                    return full
                # Extract optional
                opt_match = re.search(r'optional\s*=\s*true', full)
                is_opt = opt_match is not None
                # Extract features
                feat_match = re.search(r'features\s*=\s*(\[[^\]]*\])', full)
                feat = feat_match.group(0) if feat_match else None
                parts = [f'version = "{version}"', f'path = "{rel}"']
                if is_opt:
                    parts.append('optional = true')
                if feat:
                    parts.append(feat)
                return f'{name} = {{ {", ".join(parts)} }}'
            new_text = re.sub(simple_table_pattern, manual_replace, text)
            if new_text != text:
                text = new_text
                changed = True

    # Then handle simple form: name = "version"
    simple_pattern = rf'{name}\s*=\s*"{re.escape(version)}"'
    replacement = f'{name} = {{ version = "{version}", path = "{rel}" }}'
    new_text, count = re.subn(simple_pattern, replacement, text)
    if count:
        text = new_text
        changed = True

publish_pattern = re.compile(r'(publish\s*=\s*)true(\s*#.*)?')
new_text, count = publish_pattern.subn(lambda m: f"{m.group(1)}false{m.group(2) or ''}", text)
if count:
    text = new_text
    changed = True

if changed:
    path.write_text(text)
PY
        fi
    done

    print_success "Path dependencies restored"
}

# Function to show usage
show_usage() {
    echo "MockForge Crates Publishing Script"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --dry-run              Run in dry-run mode (don't actually publish)"
    echo "  --convert-only         Only convert dependencies, don't publish"
    echo "  --restore              Restore path dependencies for development"
    echo "  --resume               Resume publishing (skip already published crates)"
    echo "  --wait-time SECONDS    Wait time between publishes (default: 30)"
    echo "  --help                 Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  CRATES_IO_TOKEN        Your crates.io API token (required for publishing)"
    echo "  DRY_RUN                Set to 'true' for dry-run mode"
    echo "  WAIT_TIME              Wait time between publishes in seconds"
    echo ""
    echo "Examples:"
    echo "  $0 --dry-run                    # Test the publishing process"
    echo "  $0 --convert-only               # Only convert dependencies"
    echo "  $0 --restore                    # Restore development dependencies"
    echo "  $0 --resume                     # Resume publishing (skip existing crates)"
    echo "  DRY_RUN=true $0                 # Dry run using environment variable"
    echo ""
    echo "Resumable Publishing:"
    echo "  The script can be run multiple times safely. It will:"
    echo "  - Skip crates that already exist on crates.io"
    echo "  - Continue from where it left off"
    echo "  - Handle dependency conversion automatically"
}

# Main execution
main() {
    print_status "MockForge Crates Publishing Script"
    print_status "=================================="

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --convert-only)
                CONVERT_ONLY=true
                shift
                ;;
            --restore)
                RESTORE_ONLY=true
                shift
                ;;
            --resume)
                RESUME=true
                shift
                ;;
            --wait-time)
                WAIT_TIME="$2"
                shift 2
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # Handle restore-only mode
    if [ "$RESTORE_ONLY" = "true" ]; then
        restore_dependencies
        exit 0
    fi

    # Handle convert-only mode
    if [ "$CONVERT_ONLY" = "true" ]; then
        convert_dependencies
        exit 0
    fi

    # Restore all dependencies to path dependencies before starting
    # This ensures a clean state and prevents dependency resolution errors
    print_status "Restoring all dependencies to path dependencies for clean publish state..."
    restore_dependencies

    # Check for crates.io token if not in dry-run mode
    # Note: cargo publish can also use credentials from `cargo login`, so we only warn
    if [ "$DRY_RUN" = "false" ] && [ -z "$CRATES_IO_TOKEN" ] && [ -z "$CARGO_REGISTRY_TOKEN" ]; then
        print_warning "No CRATES_IO_TOKEN or CARGO_REGISTRY_TOKEN set."
        print_status "Attempting to use cargo's stored credentials (from 'cargo login')..."
        print_status "If this fails, set: export CRATES_IO_TOKEN=your_token_here"
        print_status "Get your token from: https://crates.io/me"
    fi

    # Phase 1: Publish base crates (no internal dependencies)
    print_status "Phase 1: Publishing base crates..."

    # Publish mockforge-data first (it has no dependencies on other mockforge crates)
    convert_crate_dependencies "mockforge-data"
    publish_crate "mockforge-data"
    wait_for_processing

    # Publish mockforge-template-expansion (it has no dependencies on other mockforge crates)
    convert_crate_dependencies "mockforge-template-expansion"
    publish_crate "mockforge-template-expansion"
    wait_for_processing

    # Convert dependencies for mockforge-core (can now reference mockforge-data 0.3.6 and mockforge-template-expansion 0.3.6)
    convert_crate_dependencies "mockforge-core"
    publish_crate "mockforge-core"
    wait_for_processing

    # Convert dependencies for mockforge-plugin-core and publish it
    convert_crate_dependencies "mockforge-plugin-core"
    publish_crate "mockforge-plugin-core"
    wait_for_processing

    # Publish shared internal crates required by downstream crates
    # These must be published before crates that depend on them (like mockforge-http, mockforge-plugin-sdk)
    # mockforge-tracing must be published before mockforge-observability (observability depends on tracing)
    convert_crate_dependencies "mockforge-tracing"
    publish_crate "mockforge-tracing"
    wait_for_processing

    convert_crate_dependencies "mockforge-observability"
    publish_crate "mockforge-observability"
    wait_for_processing

    # Convert dependencies for mockforge-plugin-sdk and publish it
    # (Now that observability and tracing are published)
    convert_crate_dependencies "mockforge-plugin-sdk"
    publish_crate "mockforge-plugin-sdk"
    wait_for_processing

    convert_crate_dependencies "mockforge-recorder"
    publish_crate "mockforge-recorder"
    wait_for_processing

    convert_crate_dependencies "mockforge-plugin-registry"
    publish_crate "mockforge-plugin-registry"
    wait_for_processing

    convert_crate_dependencies "mockforge-chaos"
    publish_crate "mockforge-chaos"
    wait_for_processing

    convert_crate_dependencies "mockforge-reporting"
    publish_crate "mockforge-reporting"
    wait_for_processing

    convert_crate_dependencies "mockforge-analytics"
    publish_crate "mockforge-analytics"
    wait_for_processing

    convert_crate_dependencies "mockforge-pipelines"
    publish_crate "mockforge-pipelines"
    wait_for_processing

    convert_crate_dependencies "mockforge-collab"
    publish_crate "mockforge-collab"
    wait_for_processing

    # Phase 2: Publish remaining dependent crates
    print_status "Phase 2: Publishing remaining dependent crates..."

    # Publish mockforge-performance first (required by mockforge-http)
    convert_crate_dependencies "mockforge-performance"
    publish_crate "mockforge-performance"
    wait_for_processing

    # Publish mockforge-route-chaos (required by mockforge-http)
    convert_crate_dependencies "mockforge-route-chaos"
    publish_crate "mockforge-route-chaos"
    wait_for_processing

    # Publish mockforge-world-state (required by mockforge-http)
    convert_crate_dependencies "mockforge-world-state"
    publish_crate "mockforge-world-state"
    wait_for_processing

    # Publish plugin system crates
    convert_crate_dependencies "mockforge-plugin-loader"
    publish_crate "mockforge-plugin-loader"
    wait_for_processing

    # Publish schema crate (needed by mockforge-cli)
    convert_crate_dependencies "mockforge-schema"
    publish_crate "mockforge-schema"
    wait_for_processing

    # Publish protocol crates
    # Publish dependencies of mockforge-http first (mqtt, scenarios, smtp, ws)
    convert_crate_dependencies "mockforge-mqtt"
    publish_crate "mockforge-mqtt"
    wait_for_processing

    convert_crate_dependencies "mockforge-scenarios"
    publish_crate "mockforge-scenarios"
    wait_for_processing

    convert_crate_dependencies "mockforge-smtp"
    publish_crate "mockforge-smtp"
    wait_for_processing

    convert_crate_dependencies "mockforge-ws"
    publish_crate "mockforge-ws"
    wait_for_processing

    convert_crate_dependencies "mockforge-http"
    publish_crate "mockforge-http"
    wait_for_processing

    convert_crate_dependencies "mockforge-grpc"
    publish_crate "mockforge-grpc"
    wait_for_processing

    convert_crate_dependencies "mockforge-graphql"
    publish_crate "mockforge-graphql"
    wait_for_processing

    convert_crate_dependencies "mockforge-amqp"
    publish_crate "mockforge-amqp"
    wait_for_processing

    convert_crate_dependencies "mockforge-kafka"
    publish_crate "mockforge-kafka"
    wait_for_processing

    convert_crate_dependencies "mockforge-ftp"
    publish_crate "mockforge-ftp"
    wait_for_processing

    convert_crate_dependencies "mockforge-tcp"
    publish_crate "mockforge-tcp"
    wait_for_processing

    # Publish SDK (depends on protocol crates)
    convert_crate_dependencies "mockforge-sdk"
    publish_crate "mockforge-sdk"
    wait_for_processing

    # Publish utility crates
    convert_crate_dependencies "mockforge-bench"
    publish_crate "mockforge-bench"
    wait_for_processing

    convert_crate_dependencies "mockforge-test"
    publish_crate "mockforge-test"
    wait_for_processing

    convert_crate_dependencies "mockforge-k8s-operator"
    publish_crate "mockforge-k8s-operator"
    wait_for_processing

    convert_crate_dependencies "mockforge-registry-server"
    publish_crate "mockforge-registry-server"
    wait_for_processing

    # VBR (needs to be published before mockforge-ui)
    convert_crate_dependencies "mockforge-vbr"
    publish_crate "mockforge-vbr"
    wait_for_processing

    # CLI binary (needs mockforge-ui and mockforge-tunnel published first)
    convert_crate_dependencies "mockforge-ui"
    publish_crate "mockforge-ui"
    wait_for_processing

    convert_crate_dependencies "mockforge-tunnel"
    publish_crate "mockforge-tunnel"
    wait_for_processing

    convert_crate_dependencies "mockforge-cli"
    publish_crate "mockforge-cli"
    wait_for_processing

    print_success "All crates published successfully!"

    if [ "$DRY_RUN" = "false" ]; then
        print_warning "Remember to restore path dependencies for development:"
        print_status "$0 --restore"
    fi
}

# Run main function
main "$@"

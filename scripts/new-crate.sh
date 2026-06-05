#!/usr/bin/env bash
#
# Scaffold a new mockforge-* crate already wired into every place a crate has to
# be registered, so it can't become an orphan (#796) or a publish-list drift
# (#795). Without this, a hand-rolled crate routinely misses one of:
#   - root Cargo.toml [workspace].members  -> never built/tested/linted (orphan)
#   - [lints] workspace = true             -> escapes the workspace lint set
#   - scripts/publish-crates.sh CRATES     -> publish drift if it ships
#
# Usage:
#   scripts/new-crate.sh <name> [--desc "one-line description"] [--bin] [--publish]
#
#   <name>      crate name, with or without the mockforge- prefix (e.g. "widget"
#               or "mockforge-widget")
#   --desc      crate description (default: a placeholder you should edit)
#   --bin       scaffold a binary (src/main.rs) instead of a library (src/lib.rs)
#   --publish   make it publishable now and insert it into the publish list.
#               DEFAULT is publish=false (most new crates have no published
#               consumer yet; flip later — release-guardian will flag it).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

NAME=""
DESC=""
KIND="lib"
PUBLISH=false
while [ $# -gt 0 ]; do
  case "$1" in
    --desc) DESC="$2"; shift 2 ;;
    --bin) KIND="bin"; shift ;;
    --publish) PUBLISH=true; shift ;;
    -h|--help) sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "unknown flag: $1" >&2; exit 2 ;;
    *) NAME="$1"; shift ;;
  esac
done

[ -n "$NAME" ] || { echo "error: crate name required" >&2; exit 2; }
NAME="${NAME#mockforge-}"          # strip prefix if supplied
CRATE="mockforge-$NAME"
DIR="crates/$CRATE"
[ -d "$DIR" ] && { echo "error: $DIR already exists" >&2; exit 1; }
[ -n "$DESC" ] || DESC="TODO: describe $CRATE"

echo "Scaffolding $CRATE (kind=$KIND, publish=$([ "$PUBLISH" = true ] && echo true || echo false))"

mkdir -p "$DIR/src"

# --- Cargo.toml -------------------------------------------------------------
PUBLISH_LINE=""
[ "$PUBLISH" = false ] && PUBLISH_LINE=$'# Not yet consumed by any published crate, so it does not ship to crates.io.\n# Flip to true + add to scripts/publish-crates.sh CRATES when it gains a\n# published consumer (release-guardian / the pre-push drift guard will flag it).\npublish = false\n'

cat > "$DIR/Cargo.toml" <<EOF
[package]
name = "$CRATE"
${PUBLISH_LINE}version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "$DESC"
repository.workspace = true
homepage.workspace = true
documentation.workspace = true

[dependencies]

[lints]
workspace = true
EOF

# --- source -----------------------------------------------------------------
if [ "$KIND" = bin ]; then
  cat > "$DIR/src/main.rs" <<EOF
//! $CRATE — $DESC

fn main() {
    println!("$CRATE");
}
EOF
else
  cat > "$DIR/src/lib.rs" <<EOF
//! $CRATE — $DESC
//!
//! TODO: replace this with the crate's public API. The crate-level doc comment
//! above satisfies the workspace \`missing_docs\` lint for an otherwise empty lib.
EOF
fi
# Minimal README so \`cargo publish\`/\`cargo doc\` are happy if this crate later ships.
cat > "$DIR/README.md" <<EOF
# $CRATE

$DESC
EOF

# --- register in root Cargo.toml [workspace].members ------------------------
python3 - "$CRATE" <<'PY'
import sys, pathlib, re
crate = sys.argv[1]
p = pathlib.Path("Cargo.toml")
lines = p.read_text().splitlines(keepends=True)
entry = f'    "crates/{crate}",\n'
if any(entry.strip() == l.strip() for l in lines):
    print("  members: already present"); sys.exit(0)
# insert right after the last existing "crates/mockforge-*" member line
last = None
for i, l in enumerate(lines):
    if re.match(r'\s*"crates/mockforge-[^"]+",', l):
        last = i
assert last is not None, "could not find an existing crates/mockforge-* member to anchor to"
lines.insert(last + 1, entry)
p.write_text("".join(lines))
print(f"  members: added {crate}")
PY

# --- optionally register in the publish list --------------------------------
if [ "$PUBLISH" = true ]; then
  python3 - "$CRATE" <<'PY'
import sys, pathlib, re
crate = sys.argv[1]
p = pathlib.Path("scripts/publish-crates.sh")
lines = p.read_text().splitlines(keepends=True)
if any(re.match(rf'\s*{re.escape(crate)}\s*$', l) for l in lines):
    print("  publish list: already present"); sys.exit(0)
# Insert before mockforge-cli (a final consumer that depends on ~everything),
# which is a safe topological slot for a new leaf crate. Reorder if the new
# crate is itself a dependency of something earlier.
for i, l in enumerate(lines):
    if re.match(r'\s*mockforge-cli\s*$', l):
        indent = re.match(r'(\s*)', l).group(1)
        lines.insert(i, f"{indent}{crate}\n")
        p.write_text("".join(lines))
        print(f"  publish list: inserted {crate} before mockforge-cli")
        break
else:
    print("  publish list: WARN could not find mockforge-cli anchor; add manually")
PY
fi

# --- verify -----------------------------------------------------------------
echo "Verifying..."
if cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "import sys,json;exit(0 if '$CRATE' in {p['name'] for p in json.load(sys.stdin)['packages']} else 1)"; then
  echo "  ✓ $CRATE is a workspace member"
else
  echo "  ✗ $CRATE is NOT a member — check root Cargo.toml" >&2; exit 1
fi
cargo check -p "$CRATE" 2>&1 | tail -3

cat <<EOF

Done. Next steps:
  - edit $DIR/Cargo.toml description + add dependencies
  - write code in $DIR/src/
  - if it should ship, re-run with --publish or add it to scripts/publish-crates.sh
  - run: cargo clippy -p $CRATE --all-targets -- -D warnings
EOF

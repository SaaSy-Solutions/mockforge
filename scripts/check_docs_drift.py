#!/usr/bin/env python3
"""
Docs / code drift gate.

Goal: when someone adds a new MOCKFORGE_* env var or top-level CLI subcommand
without documenting it, CI fails the PR — so user-facing docs can't silently
rot between releases.

What we check, today:

  1. **Env vars (code → docs, FAIL)**: every `MOCKFORGE_*` env var read by
     the workspace via `std::env::var(...)` appears at least once in
     user-facing docs.
  2. **CLI subcommands (code → docs, FAIL)**: every top-level subcommand in
     `Commands::Foo` (mockforge-cli main.rs) appears at least once in
     user-facing docs (kebab-case match).
  3. **Env vars (docs → code, WARN)**: every `MOCKFORGE_*` mentioned in
     user-facing docs still exists in code (or in ENV_ALLOWLIST). Reported
     as warnings (no exit) because some env vars get read indirectly via
     config-crate `Env::prefixed` patterns that this regex doesn't catch.
     Useful signal nonetheless — surfaces stale references to renamed /
     removed env vars even if it occasionally false-positives.
  4. **Per-flag coverage (code → docs, WARN)**: every `#[arg(long="...")]`
     in `Commands::*` blocks appears at least once in docs. Reported as
     warnings because clap default flags + per-protocol flag proliferation
     make the strict version too noisy.

User-facing doc roots scanned:
  - /CONFIG.md
  - /README.md
  - /CHANGELOG.md
  - /docs/**/*.md
  - /book/src/**/*.md

Internal status / planning files (those in /docs/archive/ specifically) are
*excluded* from the doc corpus — they're frozen historical artifacts and
shouldn't satisfy current-doc coverage.

Anything in the *_ALLOWLIST below is exempt; add to it with a reason if a
new symbol legitimately doesn't need user-facing docs.

Exit code 0 = no drift, 1 = drift found. Prints a punch list when it fails.

Run from the repo root:
    python3 scripts/check_docs_drift.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Env-vars in code that intentionally don't have user-facing docs (yet, or
# ever). Keep this list small and add a one-line reason for each entry.
ENV_ALLOWLIST: dict[str, str] = {
    # --- Test-only / dev-only ---
    "MOCKFORGE_TEST_FIXTURE_DIR": "test-only fixture path; not user surface",
    "MOCKFORGE_TEST_BINARY": "test-harness binary path; not user surface",
    "MOCKFORGE_DEV_JWT_SECRET": "dev-only JWT secret for local-mode auth; not for prod users",
    "MOCKFORGE_OAUTH_ACCESS_TOKEN": "test/runtime-only OAuth fixture; not user-set",
    # --- Hosted-mocks orchestrator-injected ---
    "MOCKFORGE_LOG_INGEST_URL": "hosted-mocks orchestrator-injected; not for end users",
    "MOCKFORGE_LOG_INGEST_TOKEN": "hosted-mocks orchestrator-injected; not for end users",
    "MOCKFORGE_LOG_INGEST_BATCH_SIZE": "hosted log shipper internal tunable",
    "MOCKFORGE_LOG_INGEST_BUFFER": "hosted log shipper internal tunable",
    "MOCKFORGE_LOG_INGEST_FLUSH_MS": "hosted log shipper internal tunable",
    "MOCKFORGE_CAPTURE_INGEST_URL": "hosted capture pipeline; orchestrator-injected",
    "MOCKFORGE_CAPTURE_INGEST_TOKEN": "hosted capture pipeline; orchestrator-injected",
    "MOCKFORGE_CAPTURE_INGEST_BATCH_SIZE": "hosted capture pipeline tunable",
    "MOCKFORGE_CAPTURE_INGEST_BUFFER": "hosted capture pipeline tunable",
    "MOCKFORGE_CAPTURE_INGEST_FLUSH_MS": "hosted capture pipeline tunable",
    # --- Internal infra (registry/collab/encryption plumbing) ---
    "MOCKFORGE_REGISTRY_DB_URL": "internal: registry server uses MOCKFORGE_DB_CONNECTION; this is a legacy alias",
    "MOCKFORGE_DB_KEY_DIR": "internal encryption key cache directory",
    "MOCKFORGE_DB_KEY_TABLE": "internal: per-row encryption metadata table",
    "MOCKFORGE_COMMUNITY_CONTENT_FILE": "internal: bundled community-content seed file path",
    "MOCKFORGE_PLUGIN_SCANNER_BIN": "internal: plugin scanner subprocess binary",
    # --- OSV vulnerability scanner (registry-server internal) ---
    "MOCKFORGE_OSV_FEED_URL": "registry-server internal: OSV vulnerability feed URL",
    "MOCKFORGE_OSV_SEED_PATH": "registry-server internal: OSV seed data path",
    "MOCKFORGE_OSV_ECOSYSTEMS": "registry-server internal: OSV ecosystem allowlist",
    # --- Surface aliases / partial-match misses (already documented under different names) ---
    "MOCKFORGE_RAG_ENABLED": "documented as a feature flag; some scans may miss alias",
}

# Subcommands that are intentionally hidden / dev-only and don't need a
# user-facing doc entry.
COMMAND_ALLOWLIST: dict[str, str] = {
    "Internal": "internal/dev-only group, not part of public CLI surface",
    "Debug": "developer debugging subcommand",
    "DevX": "developer-experience helper (internal)",
}

# Documented but allowed not to exist in code anymore (e.g. recently-removed
# env vars where docs still reference them as "legacy" or "previously"). Empty
# to start — add with a reason when needed.
DOC_ONLY_ENV_ALLOWLIST: dict[str, str] = {}

# Per-flag long-form flag names that don't need their own doc entry. Use
# sparingly; prefer documenting the flag.
FLAG_ALLOWLIST: dict[str, str] = {
    # Flags whose name appears in flag-table form but not as `--flag` text;
    # WARN-mode reduces the noise so most aren't actually false positives,
    # but a few legitimately don't need their own line in user-facing docs.
}

ENV_VAR_RE = re.compile(
    r"""std::env::var(?:_os)?\(\s*['"](MOCKFORGE_[A-Z0-9_]+)['"]\s*\)"""
)

# Match `MOCKFORGE_FOO_BAR` references in prose or code blocks.
DOC_ENV_VAR_RE = re.compile(r"\b(MOCKFORGE_[A-Z0-9_]+)\b")

# `Commands::Foo {`  or `Commands::Foo,` — the shape clap derive uses for
# subcommand variants on the top-level Commands enum.
COMMAND_RE = re.compile(r"^\s+([A-Z][A-Za-z0-9]+)\s*[\{,(]", re.MULTILINE)

# clap derive flag: `#[arg(long = "name")]`, `#[arg(long, short = 'p')]`,
# etc. We extract names from `long = "..."`. The shorter `long` (where the
# name is implied to be the field name) is harder to extract reliably with
# regex, so we intentionally skip those — false-negative tradeoff for
# false-positive avoidance.
FLAG_RE = re.compile(r'#\[arg\([^\]]*long\s*=\s*"([a-z][a-z0-9-]+)"')


def gather_env_vars() -> set[str]:
    """All MOCKFORGE_* env vars referenced by `std::env::var(...)` in any
    crate source file."""
    found: set[str] = set()
    for rs in REPO.glob("crates/*/src/**/*.rs"):
        try:
            text = rs.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        for m in ENV_VAR_RE.finditer(text):
            found.add(m.group(1))
    return found


def gather_subcommands() -> set[str]:
    """Top-level subcommand variants declared on the `Commands` enum in
    mockforge-cli/main.rs. We bracket the search by the `enum Commands {`
    block so we don't pick up nested enums (e.g. PluginCommands, ConfigCommands)."""
    main_rs = REPO / "crates" / "mockforge-cli" / "src" / "main.rs"
    text = main_rs.read_text(encoding="utf-8")

    start = text.find("enum Commands {")
    if start < 0:
        # Schema changed; fail loudly so we update the regex on purpose.
        sys.exit("ERROR: could not locate `enum Commands {` in mockforge-cli main.rs")
    # Find the matching closing brace at depth 0.
    depth = 0
    end = start
    for i, ch in enumerate(text[start:], start=start):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                end = i
                break
    block = text[start : end + 1]

    found: set[str] = set()
    for m in COMMAND_RE.finditer(block):
        name = m.group(1)
        # Filter out things that look like rust types rather than enum
        # variants (e.g. `Vec`, `String`, `Option`, `PathBuf`).
        if name in {"Vec", "String", "Option", "PathBuf", "HashMap", "Path", "Arc"}:
            continue
        # `Commands::*` variants always start at column 4 in clap derive style;
        # the regex anchors at column 5 onward. Filter out doc-comment artifacts.
        if name.endswith("Comment") or name.endswith("Args"):
            continue
        found.add(name)
    return found


def gather_long_flags() -> set[str]:
    """Every `#[arg(long = "...")]` declared anywhere in mockforge-cli."""
    found: set[str] = set()
    for rs in (REPO / "crates" / "mockforge-cli" / "src").rglob("*.rs"):
        try:
            text = rs.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        for m in FLAG_RE.finditer(text):
            found.add(m.group(1))
    return found


def doc_corpus() -> str:
    """Concatenate every user-facing markdown doc into one big string for
    membership tests. `docs/archive/` is excluded — those are historical
    snapshots, not current docs."""
    chunks: list[str] = []
    for top in ("CONFIG.md", "README.md", "CHANGELOG.md"):
        p = REPO / top
        if p.is_file():
            chunks.append(p.read_text(encoding="utf-8", errors="ignore"))
    for root in ("docs", "book/src"):
        for md in (REPO / root).rglob("*.md"):
            # Exclude historical / archived files from coverage.
            if "/archive/" in md.as_posix():
                continue
            try:
                chunks.append(md.read_text(encoding="utf-8", errors="ignore"))
            except OSError:
                continue
    return "\n".join(chunks)


def cli_command_to_kebab(name: str) -> str:
    """`BenchChunked` -> `bench-chunked`; that's how users actually type it
    and how the CLI surface is documented."""
    return re.sub(r"(?<!^)(?=[A-Z])", "-", name).lower()


def main() -> int:
    env_vars = gather_env_vars() - set(ENV_ALLOWLIST)
    subcommands = gather_subcommands() - set(COMMAND_ALLOWLIST)
    long_flags = gather_long_flags() - set(FLAG_ALLOWLIST)
    docs = doc_corpus()

    # --- code → docs (FAIL) ---
    missing_env = sorted(v for v in env_vars if v not in docs)
    missing_cmd = sorted(
        c for c in subcommands if cli_command_to_kebab(c) not in docs and c not in docs
    )

    # --- docs → code (FAIL) ---
    # Every MOCKFORGE_* mentioned in docs should still exist in code, OR be
    # explicitly tolerated. Skip the `MOCKFORGE_*` glob token itself (it's
    # used in prose to mean "any of these") and ones in the env allowlist
    # (already known to exist in code).
    code_env_vars = gather_env_vars() | set(ENV_ALLOWLIST)
    doc_env_mentions = {
        v
        for v in DOC_ENV_VAR_RE.findall(docs)
        if not v.endswith("_") and v not in {"MOCKFORGE_"}
    }
    stale_env = sorted(
        v
        for v in doc_env_mentions
        if v not in code_env_vars and v not in DOC_ONLY_ENV_ALLOWLIST
    )

    # --- code → docs flags (WARN) ---
    # Per-flag coverage: noisier so it's a warning, not a failure. Skip
    # ultra-common flags that ship with every subcommand and are
    # documented at the parent-command level.
    common_flags = {"help", "version", "verbose", "quiet"}
    undocumented_flags = sorted(
        f for f in long_flags if f not in common_flags and f"--{f}" not in docs
    )

    fail = bool(missing_env or missing_cmd)

    if not fail and not stale_env and not undocumented_flags:
        print("docs/code drift check: ✅ no drift")
        return 0

    if fail:
        print("docs/code drift check: ❌ missing user-facing documentation\n")

        if missing_env:
            print(f"Env vars referenced in code but not documented ({len(missing_env)}):")
            for v in missing_env:
                print(f"  - {v}")
            print(
                "\n  Fix: add a row in CONFIG.md / book/src/configuration/environment.md,"
                "\n  OR add to ENV_ALLOWLIST in scripts/check_docs_drift.py with a reason."
            )

        if missing_cmd:
            if missing_env:
                print()
            print(f"CLI subcommands declared in code but not documented ({len(missing_cmd)}):")
            for c in missing_cmd:
                print(f"  - Commands::{c}  (kebab: `mockforge {cli_command_to_kebab(c)}`)")
            print(
                "\n  Fix: add a section in book/src/api/cli.md / README.md,"
                "\n  OR add to COMMAND_ALLOWLIST in scripts/check_docs_drift.py with a reason."
            )

    if stale_env:
        if fail:
            print()
        else:
            print("docs/code drift check: ✅ no failures (warnings below)\n")
        print(
            f"WARN: env vars mentioned in docs but not found in code "
            f"({len(stale_env)}; may include false positives if read via "
            f"figment/config-crate Env::prefixed patterns):"
        )
        for v in stale_env[:30]:
            print(f"  - {v}")
        if len(stale_env) > 30:
            print(f"  ... and {len(stale_env) - 30} more")
        print(
            "\n  Investigate each: if removed/renamed, fix or remove the doc"
            "\n  reference. If a false positive (read via config crate), add it"
            "\n  to ENV_ALLOWLIST so the gate sees it as known-in-code."
        )

    if undocumented_flags:
        if fail or stale_env:
            print()
        elif not stale_env:
            print("docs/code drift check: ✅ no failures (warnings below)\n")
        print(
            f"WARN: long flags declared in code but not documented "
            f"({len(undocumented_flags)}):"
        )
        for f in undocumented_flags[:30]:
            print(f"  - --{f}")
        if len(undocumented_flags) > 30:
            print(f"  ... and {len(undocumented_flags) - 30} more")
        print(
            "\n  This is a warning (the gate doesn't fail on per-flag drift) because"
            "\n  many flags are documented at the parent-command level rather than"
            "\n  with their own line. Add doc coverage where the flag matters, or"
            "\n  add to FLAG_ALLOWLIST with a reason."
        )

    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())

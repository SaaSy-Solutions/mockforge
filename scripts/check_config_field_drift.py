#!/usr/bin/env python3
"""
Config-field drift gate.

Companion to scripts/check_docs_drift.py. That script catches missing env-var
and CLI-subcommand docs; this one catches **missing config-field docs** —
public fields on serde-derived configuration structs that users set in
mockforge.yaml / config files.

What it checks
--------------

For each (file, struct) pair in `TARGETS`, parse the struct definition,
extract `pub` field names, and require each field name to appear at least
once in user-facing docs (top-level *.md, docs/, book/src/).

What it doesn't check (yet)
---------------------------
- Nested type fields (e.g. `pub latency: LatencyConfig` is checked at the
  field name `latency`, but the LatencyConfig fields are checked separately
  if LatencyConfig is in TARGETS).
- Variant fields on enums (we only care about struct fields).
- `#[serde(skip)]`-annotated fields are skipped (correct behavior — they're
  not user-set).

Allowlist
---------
`FIELD_ALLOWLIST` exempts fields that are public for cross-crate access but
not user-facing. Add an entry only with a clear reason.

Run from the repo root:
    python3 scripts/check_config_field_drift.py
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


@dataclass(frozen=True)
class Target:
    file: str  # repo-relative
    struct: str  # struct name to parse


# Structs whose public fields users set in YAML/JSON config. Order doesn't
# matter — each is checked independently.
TARGETS: list[Target] = [
    # Chaos engine
    Target("crates/mockforge-chaos/src/config.rs", "ChaosConfig"),
    Target("crates/mockforge-chaos/src/config.rs", "FaultInjectionConfig"),
    Target("crates/mockforge-chaos/src/config.rs", "LatencyConfig"),
    Target("crates/mockforge-chaos/src/config.rs", "RateLimitConfig"),
    Target("crates/mockforge-chaos/src/config.rs", "TrafficShapingConfig"),
    Target("crates/mockforge-chaos/src/config.rs", "CircuitBreakerConfig"),
    Target("crates/mockforge-chaos/src/config.rs", "BulkheadConfig"),
    Target("crates/mockforge-chaos/src/request_matcher.rs", "RequestMatcher"),
    Target("crates/mockforge-chaos/src/request_matcher.rs", "HeaderMatch"),
]


# Per-(struct, field) allowlist. Use sparingly with a reason.
# The key is "<StructName>.<field_name>".
FIELD_ALLOWLIST: dict[str, str] = {
    # Internal Prometheus metric labels / debug knobs that aren't user-set.
    # (Currently empty — populate as we add structs that have non-user fields.)
}


# Match `pub struct Foo {` allowing arbitrary attributes / docs above it.
def find_struct_block(text: str, struct: str) -> str | None:
    """Return the body (between `{` and matching `}`) of `pub struct <name>`,
    or None if not found. Handles only struct-with-named-fields form."""
    pattern = re.compile(
        rf"pub\s+struct\s+{re.escape(struct)}\s*(?:<[^>]*>)?\s*\{{", re.MULTILINE
    )
    m = pattern.search(text)
    if m is None:
        return None
    start = m.end() - 1  # position of `{`
    depth = 0
    for i, ch in enumerate(text[start:], start=start):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
    return None


def extract_pub_fields(block: str) -> list[str]:
    """Walk the struct body line-by-line and return each `pub <name>:` field
    name. Skips fields whose nearest preceding non-blank attribute line
    contains `#[serde(skip)]` or `skip_serializing` (fields that won't be
    set by users)."""
    out: list[str] = []
    pending_attrs: list[str] = []
    for raw in block.splitlines():
        line = raw.strip()
        if not line or line.startswith("//"):
            continue
        if line.startswith("#["):
            pending_attrs.append(line)
            continue
        m = re.match(r"pub\s+(\w+)\s*:", line)
        if m:
            field = m.group(1)
            skip = any(
                "skip)" in a or "skip_serializing" in a or 'skip,' in a
                for a in pending_attrs
            )
            if not skip:
                out.append(field)
        # Reset pending attrs after any non-attribute line.
        pending_attrs = []
    return out


def doc_corpus() -> str:
    """Same corpus as the env-var drift gate. Concatenated for one membership
    test pass."""
    chunks: list[str] = []
    for top in ("CONFIG.md", "README.md", "CHANGELOG.md"):
        p = REPO / top
        if p.is_file():
            chunks.append(p.read_text(encoding="utf-8", errors="ignore"))
    for root in ("docs", "book/src"):
        for md in (REPO / root).rglob("*.md"):
            try:
                chunks.append(md.read_text(encoding="utf-8", errors="ignore"))
            except OSError:
                continue
    return "\n".join(chunks)


def main() -> int:
    docs = doc_corpus()
    missing: list[tuple[str, str, str]] = []  # (struct, field, file)
    parsed = 0

    for t in TARGETS:
        path = REPO / t.file
        if not path.is_file():
            print(f"WARN: target file missing: {t.file}", file=sys.stderr)
            continue
        text = path.read_text(encoding="utf-8")
        block = find_struct_block(text, t.struct)
        if block is None:
            print(f"WARN: struct {t.struct!r} not found in {t.file}", file=sys.stderr)
            continue
        parsed += 1
        for field in extract_pub_fields(block):
            key = f"{t.struct}.{field}"
            if key in FIELD_ALLOWLIST:
                continue
            if field not in docs:
                missing.append((t.struct, field, t.file))

    if not missing:
        print(f"config-field drift check: ✅ no drift ({parsed} structs parsed)")
        return 0

    print("config-field drift check: ❌ undocumented public config fields\n")
    by_struct: dict[str, list[tuple[str, str]]] = {}
    for struct, field, file in missing:
        by_struct.setdefault(struct, []).append((field, file))

    for struct in sorted(by_struct):
        items = by_struct[struct]
        print(f"  {struct} ({len(items)} field(s) in {items[0][1]}):")
        for field, _ in items:
            print(f"    - {field}")

    print(
        "\n  Fix: document each field in CONFIG.md, "
        "book/src/configuration/*.md, or the chapter where the struct's"
        "\n  feature lives. OR add to FIELD_ALLOWLIST in"
        "\n  scripts/check_config_field_drift.py with a reason."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())

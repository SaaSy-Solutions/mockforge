#!/usr/bin/env python3
"""Build the tools shipped inside the crates.io CLI; --check detects stale bundles."""

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "crates/mockforge-cli/src/bundled_tools.json"


def bundle():
    paths = [
        ROOT / "scripts/export_github_issue.py",
        ROOT / "scripts/security_rule_test_pack.py",
    ]
    paths += sorted((ROOT / "scripts").glob("cve_pack*.py"))
    paths += sorted((ROOT / "scripts").glob("security_rule_pack_*.py"))
    for directory in ("cve-packs", "cve-lab", "security-rule-packs"):
        paths += sorted(
            p
            for p in (ROOT / "examples" / directory).rglob("*")
            if p.is_file() and p.suffix in {".json", ".yaml", ".md"}
        )
    paths += [
        ROOT / "book/src/user-guide" / name
        for name in (
            "cve-packs.md",
            "issue-79-reference.md",
            "security-rule-test-packs.md",
        )
    ]
    files = {str(p.relative_to(ROOT)): p.read_text(encoding="utf-8") for p in paths}
    files["README.md"] = (
        "# MockForge discussion and CVE tools\n\n"
        "Requires Python 3.11 or later; no pip packages or Git checkout needed.\n"
        "Run these commands from this directory:\n\n"
        "```bash\n"
        "python3 scripts/export_github_issue.py --repo SaaSy-Solutions/mockforge "
        "--issue 79 --output-dir issue-79-export\n"
        "python3 scripts/cve_pack.py --help\n"
        "python3 scripts/cve_pack.py tips CVE-2026-109618\n"
        "python3 scripts/cve_pack.py server "
        "examples/cve-packs/CVE-2025-29927/profile.json > mock.json\n"
        "```\n\n"
        "See book/src/user-guide/cve-packs.md for the LB recorder setup, "
        "run versus probe, and limitations. A CVE ID creates a template, "
        "not a vulnerable application or exploit.\n"
    )
    return json.dumps(files, ensure_ascii=False, sort_keys=True, indent=2) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = bundle()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != expected:
            parser.exit(
                1,
                "CLI tools bundle is stale; run python3 scripts/bundle_cli_tools.py\n",
            )
    else:
        OUTPUT.write_text(expected, encoding="utf-8")


if __name__ == "__main__":
    main()

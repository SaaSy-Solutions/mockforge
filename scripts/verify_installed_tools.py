#!/usr/bin/env python3
"""Exercise tools from an installed MockForge binary outside the source checkout."""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def verify(binary):
    binary = str(Path(binary).resolve())
    with tempfile.TemporaryDirectory(prefix="mockforge-installed-tools-") as directory:
        root = Path(directory)

        def run(*args, code=0):
            result = subprocess.run(
                args, cwd=root, capture_output=True, text=True, timeout=30
            )
            if result.returncode != code:
                raise AssertionError(
                    f"{args}: expected exit {code}, got {result.returncode}\n{result.stdout}\n{result.stderr}"
                )
            return result.stdout

        run(binary, "tools", "export", "--output-dir", "tools")
        version = run(binary, "--version").strip().split()[-1]
        assert (root / "tools/VERSION").read_text().strip() == version
        readme = root / "tools/README.md"
        readme.write_text("user edits", encoding="utf-8")
        run(binary, "tools", "export", "--output-dir", "tools", code=1)
        assert readme.read_text() == "user edits"
        pack = str(root / "tools/scripts/cve_pack.py")
        exporter = str(root / "tools/scripts/export_github_issue.py")
        run(sys.executable, pack, "new", "CVE-2026-109618", "--output", "draft.json")
        run(sys.executable, pack, "server", "draft.json", code=2)
        spec = json.loads(
            run(
                sys.executable,
                pack,
                "server",
                "tools/examples/cve-packs/CVE-2025-29927/profile.json",
            )
        )
        assert "/dashboard" in spec["paths"]
        run(sys.executable, str(root / "tools/scripts/cve_pack_monitor.py"), "--help")
        issue_url = "https://github.com/SaaSy-Solutions/mockforge/issues/79"
        snapshot = {
            "repository": "SaaSy-Solutions/mockforge",
            "exported_at": "2026-10-30T00:00:00Z",
            "issue": {
                "number": 79,
                "title": "Installed tools",
                "html_url": issue_url,
                "body": "Searchable from a crates.io install",
                "comments": 0,
                "state": "open",
                "user": {"login": "reporter"},
                "created_at": "2026-10-30T00:00:00Z",
                "updated_at": "2026-10-30T00:00:00Z",
            },
            "comments": [],
        }
        (root / "input.json").write_text(json.dumps(snapshot), encoding="utf-8")
        run(
            sys.executable,
            exporter,
            "--input",
            "input.json",
            "--output-dir",
            "discussion",
        )
        assert (
            "Searchable from a crates.io install"
            in (root / "discussion/discussion.html").read_text()
        )
        assert json.loads((root / "discussion/snapshot.json").read_text()) == snapshot
        print(
            f"MockForge {version}: installed tools export and offline workflows passed"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True)
    verify(parser.parse_args().binary)

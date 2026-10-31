#!/usr/bin/env python3
"""Resolve the real workspace patch from each Rust builder's copied inputs.

This is a metadata-only regression, not a container image build or Rust compile.
"""
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PATCH = tomllib.loads((ROOT / "Cargo.toml").read_text())["patch"]["crates-io"]["aws-sigv4"]["path"]


def rust_builders():
    files = list(ROOT.glob("Dockerfile*")) + list((ROOT / "crates").glob("*/Dockerfile"))
    return [path for path in files if re.search(r"^COPY Cargo\.toml\b", path.read_text(), re.M)]


def metadata_from_builder(builder):
    text = builder.read_text()
    manifest_copy = re.search(r"^COPY Cargo\.toml\b", text, re.M).start()
    instructions = text[manifest_copy:].split("\nRUN cargo ", 1)[0]
    vendor_copied = re.search(r"^COPY vendor/aws-sigv4/?\s+\./vendor/aws-sigv4/?\s*$", instructions, re.M)
    with tempfile.TemporaryDirectory(prefix="sigv4a-container-inputs-") as directory:
        context = Path(directory)
        (context / "src").mkdir()
        (context / "src/lib.rs").write_text("")
        (context / "Cargo.toml").write_text(
            '[package]\nname="container-patch-inputs"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[patch.crates-io]\naws-sigv4={path="' + PATCH + '"}\n'
        )
        if vendor_copied:
            (context / "vendor").mkdir()
            (context / PATCH).symlink_to(ROOT / PATCH, target_is_directory=True)
        environment = dict(os.environ, CARGO_TARGET_DIR=str(context / "target"))
        return subprocess.run(
            ["cargo", "metadata", "--offline", "--format-version", "1", "--manifest-path", str(context / "Cargo.toml")],
            cwd=ROOT, env=environment, capture_output=True, text=True, timeout=30,
        )


class Sigv4aContainerInputs(unittest.TestCase):
    def test_each_rust_builder_can_resolve_its_workspace_patch(self):
        builders = rust_builders()
        self.assertTrue(builders, "No Rust Docker builders discovered; this is not a passing empty test")
        for builder in builders:
            with self.subTest(builder=str(builder.relative_to(ROOT))):
                result = metadata_from_builder(builder)
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Compare this extraction with its pinned Git baseline, not an ongoing freeze."""

from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
BASELINE = "3871677f241ee8e9726ad7646fa356c0a2df04ca"
CORE = "crates/mockforge-core/src"
PROTOCOL = "crates/mockforge-protocol/src"
FILES = (
    "protocol_abstraction/mod.rs",
    "protocol_abstraction/base_registry.rs",
    "protocol_abstraction/matcher.rs",
    "protocol_abstraction/protocol_registry.rs",
    "protocol_abstraction/streaming.rs",
    "fixture_store.rs",
    "protocol_server.rs",
)
WIRING = (
    "pub mod auth;\n",
    "pub mod middleware;\n",
    "pub use auth::{AuthMiddleware, AuthResult, Claims};\n",
    "pub use middleware::{LatencyMiddleware, LoggingMiddleware, MetricsMiddleware};\n",
)


def original(path):
    return subprocess.check_output(
        ["git", "show", f"{BASELINE}:{path}"], cwd=ROOT, text=True
    )


def require(path, expected):
    if (ROOT / path).read_text() != expected:
        raise SystemExit(f"Source identity mismatch: {path}")


def check_moves():
    for relative in FILES:
        text = original(f"{CORE}/{relative}")
        if relative == "protocol_abstraction/mod.rs":
            for line in WIRING:
                if text.count(line) != 1:
                    raise SystemExit(f"Baseline adapter wiring changed: {line!r}")
                text = text.replace(line, "")
        if relative in ("fixture_store.rs", "protocol_server.rs"):
            expected_count = 1 if relative == "fixture_store.rs" else 2
            if text.count("mockforge_core::") != expected_count:
                raise SystemExit("Baseline documentation import count changed")
            text = text.replace("mockforge_core::", "mockforge_protocol::")
        require(f"{PROTOCOL}/{relative}", text)
        if relative not in ("protocol_abstraction/mod.rs", "fixture_store.rs", "protocol_server.rs"):
            if (ROOT / CORE / relative).exists():
                raise SystemExit(f"Duplicate original protocol source: {relative}")
    expected = {f"{PROTOCOL}/{path}" for path in FILES} | {f"{PROTOCOL}/lib.rs"}
    actual = {str(p.relative_to(ROOT)) for p in (ROOT / PROTOCOL).rglob("*.rs")}
    if actual != expected:
        raise SystemExit(f"Unexpected protocol source inventory: {actual ^ expected}")


def check_transports():
    count = 0
    for package in ("mockforge-tcp", "mockforge-smtp"):
        prefix = f"crates/{package}"
        files = subprocess.check_output(
            ["git", "ls-tree", "-r", "--name-only", BASELINE, "--", prefix],
            cwd=ROOT, text=True,
        ).splitlines()
        rust_files = {path for path in files if path.endswith(".rs")}
        actual = {str(p.relative_to(ROOT)) for p in (ROOT / prefix).rglob("*.rs")}
        if actual != rust_files:
            raise SystemExit(f"Transport source inventory changed: {package}")
        for path in files:
            if path.endswith(".rs"):
                expected = original(path).replace("mockforge_core::", "mockforge_protocol::")
                if path == "crates/mockforge-tcp/src/server.rs":
                    before = (
                        "base64::engine::general_purpose::STANDARD\n"
                        "                .decode(&response.data)\n"
                        '                .map_err(|e| mockforge_protocol::Error::internal(format!("Invalid base64 data: {e}")))'
                    )
                    after = (
                        "base64::engine::general_purpose::STANDARD.decode(&response.data).map_err(|e| {\n"
                        '                mockforge_protocol::Error::internal(format!("Invalid base64 data: {e}"))\n'
                        "            })"
                    )
                    if expected.count(before) != 1:
                        raise SystemExit("Baseline TCP formatting exception changed")
                    expected = expected.replace(before, after)
                require(path, expected)
                count += 1
        before = tomllib.loads(original(f"{prefix}/Cargo.toml"))
        after = tomllib.loads((ROOT / prefix / "Cargo.toml").read_text())
        before["dependencies"].pop("mockforge-core")
        dependency = after["dependencies"].pop("mockforge-protocol")
        if dependency != {
            "path": "../mockforge-protocol", "version": "0.3.240",
            "features": ["scripting"],
        }:
            raise SystemExit(f"Transport feature contract changed: {package}")
        if before != after:
            raise SystemExit(f"Unexpected transport manifest edits: {package}")
    return count


def check_protocol_manifest():
    manifest = tomllib.loads((ROOT / "crates/mockforge-protocol/Cargo.toml").read_text())
    if manifest["features"] != {
        "default": [], "scripting": ["mockforge-foundation/scripting"],
        "schema": ["mockforge-foundation/schema"],
    }:
        raise SystemExit("Protocol feature contract changed")
    if manifest["dependencies"]["mockforge-foundation"].get("default-features") is not False:
        raise SystemExit("Foundation must remain default-off")
    check_back_edges(manifest)
    for target in manifest.get("target", {}).values():
        check_back_edges(target)
    if manifest["lints"]["rust"]["missing_docs"] != "deny":
        raise SystemExit("Preserve core's missing_docs denial")


def check_back_edges(table):
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, dependency in table.get(section, {}).items():
            actual_name = dependency.get("package", name) if isinstance(dependency, dict) else name
            if actual_name in {"mockforge-core", "mockforge-intelligence", "mockforge-openapi"}:
                raise SystemExit(f"Forbidden protocol back edge: {actual_name}")


def check_core_wiring():
    before = tomllib.loads(original("crates/mockforge-core/Cargo.toml"))
    after = tomllib.loads((ROOT / "crates/mockforge-core/Cargo.toml").read_text())
    dependency = after["dependencies"].pop("mockforge-protocol")
    if dependency != {"path": "../mockforge-protocol", "default-features": False}:
        raise SystemExit("Core protocol dependency must remain default-off")
    if before != after:
        raise SystemExit("Unexpected core manifest or feature changes")
    for path in ("lib.rs", "protocol_abstraction/auth.rs", "protocol_abstraction/middleware.rs"):
        require(f"{CORE}/{path}", original(f"{CORE}/{path}"))
    require(f"{CORE}/protocol_abstraction/mod.rs", (
        "//! Protocol contracts with core-hosted authentication, logging and latency adapters.\n\n"
        "pub use mockforge_protocol::protocol_abstraction::*;\n\n"
        "pub mod auth;\npub mod middleware;\n\n"
        "pub use auth::{AuthMiddleware, AuthResult, Claims};\n"
        "pub use middleware::{LatencyMiddleware, LoggingMiddleware, MetricsMiddleware};\n"
    ))
    for module, description in (
        ("fixture_store", "protocol fixture loading"),
        ("protocol_server", "protocol server lifecycle management"),
    ):
        require(f"{CORE}/{module}.rs", (
            f"//! Compatibility exports for {description}.\n\n"
            f"pub use mockforge_protocol::{module}::*;\n"
        ))
    require(f"{PROTOCOL}/lib.rs", (
        "//! Protocol-neutral contracts, registries, matching, fixture loading and server lifecycles.\n"
        "//!\n//! Authentication, global logging and latency adapters remain in `mockforge-core`.\n\n"
        "pub mod fixture_store;\npub mod protocol_abstraction;\npub mod protocol_server;\n\n"
        "pub use mockforge_foundation::{Error, Result};\npub use protocol_abstraction::*;\n"
    ))


def main():
    check_moves()
    count = check_transports()
    check_protocol_manifest()
    check_core_wiring()
    print(f"Source identity: {len(FILES)} moved files and {count} transport files; features preserved")


if __name__ == "__main__":
    main()

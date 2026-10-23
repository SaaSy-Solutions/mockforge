"""Validated host policy schema and conservative Cargo workload classification."""

from __future__ import annotations
import os
import re
from pathlib import Path

CLASSES = ("check", "targeted-test", "executable", "workspace", "release")
FIELDS = {
    "schema_version",
    "max_concurrent",
    "reserved_checks",
    "memory_budget_mib",
    "cpu_budget_percent",
    "classes",
}
V2_FIELDS = FIELDS | {"light_checks", "minimum_available_mib"}
ADMISSION_CLASSES = {"LIGHT_CHECK", "STANDARD_CHECK", "HEAVY_BUILD"}
LIMITS = {"memory_mib", "cpu_percent", "jobs"}


def host_budgets():
    memory = (
        int(
            next(
                line.split()[1]
                for line in Path("/proc/meminfo").read_text().splitlines()
                if line.startswith("MemTotal:")
            )
        )
        // 1024
    )
    return min(65536, memory * 4 // 5), max(100, len(os.sched_getaffinity(0)) * 80)


def default_policy():
    memory, cpu = host_budgets()
    memory, cpu = min(49152, memory), min(800, cpu)
    slots = max(1, min(4, cpu // 200, memory // 12288))
    return {
        "schema_version": 2,
        "max_concurrent": slots,
        "reserved_checks": max(0, slots - 2),
        "minimum_available_mib": min(16384, max(256, memory // 3)),
        "light_checks": [
            {
                "repository": "SaaSy-Solutions/aether",
                "arguments": ["check", "-p", "founderos-proto", "--lib"],
                "evidence": (
                    "saas-platform/docs/development/"
                    "rust-build-performance-2026-10-22/SSD_RECALIBRATION.md"
                ),
            }
        ],
        "memory_budget_mib": memory,
        "cpu_budget_percent": cpu,
        "classes": {
            name: {
                "memory_mib": min(12288, memory // slots),
                "cpu_percent": min(200, cpu // slots),
                "jobs": min(2, len(os.sched_getaffinity(0))),
            }
            for name in CLASSES
        },
    }


def integer(value, low, high, label):
    if type(value) is not int or not low <= value <= high:
        raise ValueError(f"{label} must be an integer in {low}..{high}")


def validate(value):
    if (
        not isinstance(value, dict)
        or value.get("schema_version") not in {1, 2}
        or set(value) != (V2_FIELDS if value.get("schema_version") == 2 else FIELDS)
    ):
        raise ValueError("Unsupported or malformed build policy")
    integer(value["schema_version"], 1, 2, "schema_version")
    integer(value["max_concurrent"], 1, 8, "max_concurrent")
    integer(value["reserved_checks"], 0, value["max_concurrent"] - 1, "reserved_checks")
    memory_budget, cpu_budget = host_budgets()
    integer(value["memory_budget_mib"], 1024, memory_budget, "memory_budget_mib")
    integer(value["cpu_budget_percent"], 100, cpu_budget, "cpu_budget_percent")
    if not isinstance(value["classes"], dict) or set(value["classes"]) != set(CLASSES):
        raise ValueError("Policy must specify every workload class")
    for name, limits in value["classes"].items():
        if not isinstance(limits, dict) or set(limits) != LIMITS:
            raise ValueError(f"Malformed limits for {name}")
        integer(limits["memory_mib"], 256, 32768, "memory_mib")
        integer(limits["cpu_percent"], 100, 800, "cpu_percent")
        integer(limits["jobs"], 1, 8, "jobs")
    for field, budget in (
        ("memory_mib", "memory_budget_mib"),
        ("cpu_percent", "cpu_budget_percent"),
    ):
        if (
            value["max_concurrent"]
            * max(item[field] for item in value["classes"].values())
            > value[budget]
        ):
            raise ValueError(f"Worst-case concurrent {field} exceeds {budget}")
    if value["schema_version"] == 2:
        validate_recipes(value, memory_budget)
    return value


def validate_recipes(value, memory_budget):
    integer(value["minimum_available_mib"], 128, memory_budget, "minimum_available_mib")
    rules = value["light_checks"]
    if not isinstance(rules, list) or len(rules) > 64:
        raise ValueError("light_checks must be a bounded list of measured recipes")
    for rule in rules:
        if not isinstance(rule, dict) or set(rule) != {
            "repository",
            "arguments",
            "evidence",
        }:
            raise ValueError("Malformed measured recipe")
        if not all(
            isinstance(rule[k], str) and 1 <= len(rule[k]) <= 512
            for k in ("repository", "evidence")
        ):
            raise ValueError(
                "Recipe requires repository identity and measurement evidence"
            )
        args = rule["arguments"]
        if (
            not isinstance(args, list)
            or not 3 <= len(args) <= 32
            or not all(isinstance(a, str) and 0 < len(a) <= 256 for a in args)
            or args[:2] != ["check", "-p"]
            or not re.fullmatch(r"[A-Za-z0-9_-]+", args[2])
            or "--lib" not in args
            or any(
                a
                in {
                    "-p",
                    "--package",
                    "--all-targets",
                    "--bins",
                    "--tests",
                    "--examples",
                    "--benches",
                    "--bin",
                    "--test",
                    "--example",
                    "--bench",
                    "--target",
                }
                or a.startswith(
                    (
                        "--package=",
                        "--target=",
                        "--bin=",
                        "--test=",
                        "--example=",
                        "--bench=",
                    )
                )
                for a in args[3:]
            )
            or classify(args) != "check"
        ):
            raise ValueError("Only explicit targeted check recipes can be lightweight")


def classify(arguments):
    args = list(arguments)
    args = args[: args.index("--")] if "--" in args else args
    verb = next(
        (
            a
            for a in args
            if a
            in {
                "check",
                "c",
                "clippy",
                "test",
                "t",
                "build",
                "b",
                "run",
                "r",
                "bench",
                "doc",
                "rustc",
                "rustdoc",
            }
        ),
        "",
    )
    profiles = []
    for index, arg in enumerate(args):
        if arg.startswith("--profile="):
            profiles.append(arg.split("=", 1)[1])
        elif arg == "--profile":
            profiles.append(args[index + 1] if index + 1 < len(args) else "unknown")
    if (
        "--release" in args
        or "-r" in args
        or verb == "bench"
        or any(p not in {"dev", "test"} for p in profiles)
    ):
        return "release"
    if any(a in {"--workspace", "--all", "--all-features"} for a in args):
        return "workspace"
    package = any(
        a in {"-p", "--package"}
        or a.startswith("--package=")
        or (a.startswith("-p") and len(a) > 2)
        for a in args
    )
    if not package:
        return "workspace"
    if verb in {"check", "c", "clippy"}:
        return "check"
    if verb in {"test", "t"}:
        return "targeted-test"
    return "executable"

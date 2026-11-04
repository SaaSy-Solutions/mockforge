#!/usr/bin/env python3
"""Opt-in UI input-selector controls, not a timing or complete frontend benchmark."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CLIENT = ["cargo", "rust_build_policy.py", "rust_policy_config.py",
          "rust_policy_runtime.py", "rust_policy_warmth.py"]
FRONTEND_INPUTS = ["package.json", "pnpm-lock.yaml", "index.html", "vite.config.ts",
                   "postcss.config.js", "tailwind.config.js", "tsconfig.json",
                   "tsconfig.app.json", "tsconfig.node.json"]
CHANGES = ["create-index", "edit-index", "remove-index", "create-icon", "edit-icon",
           "remove-icon", "remove-public", "recreate-public", "edit-source-config",
           "remove-source-config", "recreate-source-config", "edit-source-index",
           "remove-source-index", "recreate-source-index", "edit-lock", "node-modules-only"]
STEPS = ["initial", "noop"] + [step for change in CHANGES for step in (change, "noop")]
BUILD_SCRIPT = '''mod build_support;
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for input in build_support::watched_inputs(root).expect("UI anchor") {
        println!("cargo:rerun-if-changed={}", input.display());
    }
    // Exercise generated watched files after invocation starts, even when the
    // synthetic output bytes are identical. This is never an actual UI bundle.
    std::fs::write(root.join("ui/dist/build-produced.html"), "synthetic build output")
        .expect("owned synthetic dist output");
}
'''


def prepare_fixture(fixture):
    (fixture / "scripts/dev").mkdir(parents=True)
    identities = {}
    for name in CLIENT:
        source = ROOT / "scripts/dev" / name
        destination = fixture / "scripts/dev" / name
        shutil.copyfile(source, destination)
        destination.chmod(source.stat().st_mode & 0o777)
        if source.read_bytes() != destination.read_bytes():
            raise ValueError(f"launcher copy differs: {name}")
        identities[name] = hashlib.sha256(destination.read_bytes()).hexdigest()
    support = (ROOT / "crates/mockforge-ui/build_support.rs").read_bytes()
    (fixture / "build_support.rs").write_bytes(support)
    identities["build_support.rs"] = hashlib.sha256(support).hexdigest()
    (fixture / "Cargo.toml").write_text('[package]\nname="mockforge-ui-watch-probe"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[lib]\npath="build_support.rs"\n')
    (fixture / "Cargo.lock").write_text('version = 3\n[[package]]\nname = "mockforge-ui-watch-probe"\nversion = "0.0.0"\n')
    (fixture / "build.rs").write_text(BUILD_SCRIPT)
    for name in ["src", "dist", "public", "node_modules"]:
        (fixture / "ui" / name).mkdir(parents=True)
    for name in FRONTEND_INPUTS:
        (fixture / "ui" / name).write_text("synthetic source input")
    subprocess.run(["git", "init", "-q", str(fixture)], check=True)
    return {"sha256": identities, "canonical_target": str(fixture / "target"),
            "build_script_sha256": hashlib.sha256(BUILD_SCRIPT.encode()).hexdigest(),
            "rustc": subprocess.check_output(["rustc", "-vV"], cwd=fixture, text=True)}


def invoke(fixture, evidence, label, operation):
    command = [str(fixture / "scripts/dev/cargo"), operation, "--locked", "--lib",
               "--message-format=json"]
    with (evidence / f"{label}.jsonl").open("w") as stdout, (evidence / f"{label}.stderr").open("w") as stderr:
        subprocess.run(command, cwd=fixture, stdout=stdout, stderr=stderr, check=True, timeout=600)
    lines = (evidence / f"{label}.jsonl").read_text().splitlines()
    messages = [json.loads(line) for line in lines if line.startswith("{")]
    receipts = [json.loads(line) for line in (evidence / f"{label}.stderr").read_text().splitlines()
                if line.startswith("{") and '"policy_sha256"' in line]
    if len(receipts) != 1 or receipts[0]["exit_status"] != 0:
        raise ValueError("missing successful ordinary admission receipt")
    artifacts = [m for m in messages if m.get("reason") == "compiler-artifact"
                 and m["target"]["name"] == "mockforge_ui_watch_probe"
                 and "lib" in m["target"]["kind"]]
    scripts = [m for m in messages if m.get("reason") == "build-script-executed"]
    if len(artifacts) != 1 or len(scripts) != 1:
        raise ValueError("fixture artifact identity is missing or ambiguous")
    out_dir = Path(scripts[0]["out_dir"]).resolve()
    if not out_dir.is_relative_to(fixture / "target"):
        raise ValueError("artifact escaped its own canonical target")
    return {"step": label, "fresh": artifacts[0]["fresh"],
            "script_output_mtime_ns": (out_dir.parent / "output").stat().st_mtime_ns,
            "admission": receipts[0]}, lines


def mutate(fixture, step):
    ui = fixture / "ui"
    index, icon = ui / "dist/index.html", ui / "public/mockforge-icon.png"
    config, source_index = ui / "vite.config.ts", ui / "index.html"
    actions = {
        "create-index": lambda: index.write_text("first"),
        "edit-index": lambda: index.write_text("second"),
        "remove-index": index.unlink,
        "create-icon": lambda: icon.write_bytes(b"first"),
        "edit-icon": lambda: icon.write_bytes(b"second"),
        "remove-icon": icon.unlink,
        "remove-public": icon.parent.rmdir,
        "recreate-public": icon.parent.mkdir,
        "edit-source-config": lambda: config.write_text("changed config"),
        "remove-source-config": config.unlink,
        "recreate-source-config": lambda: config.write_text("restored config"),
        "edit-source-index": lambda: source_index.write_text("changed HTML input"),
        "remove-source-index": source_index.unlink,
        "recreate-source-index": lambda: source_index.write_text("restored HTML input"),
        "edit-lock": lambda: (ui / "pnpm-lock.yaml").write_text("changed lock input"),
        "node-modules-only": lambda: (ui / "node_modules/irrelevant.txt").write_text("not an input"),
    }
    actions.get(step, lambda: None)()


def qualify_transitions(fixture, evidence, records):
    for number, step in enumerate(STEPS):
        mutate(fixture, step)
        result, _ = invoke(fixture, evidence, f"{number:02}-{step}", "check")
        expect_fresh = step in {"noop", "node-modules-only"}
        if result["fresh"] != expect_fresh:
            raise ValueError(f"unexpected freshness: {result}")
        if records:
            reran = result["script_output_mtime_ns"] != records[-1]["script_output_mtime_ns"]
            if reran == expect_fresh:
                raise ValueError(f"unexpected build-script execution: {result}")
        records.append(result)


def run(evidence):
    if not evidence.is_absolute() or Path("/tmp") not in evidence.resolve().parents:
        raise ValueError("use an explicitly new owned /tmp evidence directory")
    evidence.mkdir(exist_ok=False)
    records = []
    target = ROOT / "target"
    if target.is_symlink():
        raise ValueError("refusing a symlink target")
    target.mkdir(exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix="ui-watch-probe-", dir=target) as folder:
            fixture = Path(folder)
            identity = prepare_fixture(fixture)
            (evidence / "identities.json").write_text(json.dumps(identity, indent=2) + "\n")
            _, lines = invoke(fixture, evidence, "native-tests", "test")
            if not any("test result: ok. 6 passed; 0 failed" in line for line in lines):
                raise ValueError("six native helper controls did not all execute successfully")
            qualify_transitions(fixture, evidence, records)
    finally:
        (evidence / "results.json").write_text(json.dumps(records, indent=2) + "\n")
    print(f"Six native controls and {len(records)} actual Cargo transitions passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if not args.run:
        print("No Cargo invoked. Planned transitions: " + json.dumps(STEPS))
    elif args.output is None:
        parser.error("--run requires --output")
    else:
        run(args.output)


if __name__ == "__main__":
    main()

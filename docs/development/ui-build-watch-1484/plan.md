# UI build-input watch repair

Issue [#1484](https://github.com/SaaSy-Solutions/mockforge/issues/1484) addresses a confirmed no-change UI rebuild at source `2794bcdebd5bd753e8b1c82e5f307f84da0ec08b`. Both real guarded UI checks passed. The repeat reused 956 of 957 artifacts but rebuilt `mockforge_ui`. Cargo fingerprint diagnostics explicitly report missing `crates/mockforge-ui/ui/build.rs`; the actual script is crate-root `build.rs`. Its output timestamp changed on the unchanged repeat.

The other absent watches are `ui/dist/index.html`, `ui/dist/manifest.json`, and `ui/dist/assets`. Removing only the bogus script path leaves the same problem for optional UI assets. The earlier TCP repeat stayed completely fresh; this finding concerns the UI recipe.

## Chosen boundary

Keep the existing Git/build timestamp, frontend build invocation, placeholder HTML, and asset-selection code. Add a small standard-library-only build-support module that computes existing watch paths. Watch actual `build.rs`, existing `ui/src`, `ui/dist`, and `ui/public` directories. For absent source/public paths in the published layout, watch their existing UI parent so future files and directories still invalidate. Existing main code already creates `ui/dist` before generating assets. Never create fake index HTML or JSON metadata to satisfy Cargo watches.

Watch the optional source-only `build_ui.sh` when it exists; the published package intentionally excludes that script. Include the new build-support source in the package. Native controls must cover the source layout, published layout, absent assets becoming present, edits/removals, no-op repeats, and no watch target below Cargo output. Generated-file write suppression is an optional follow-on only if an actual control demonstrates a need; it does not fix a missing watch or justify changing version timestamps.

The alternative is one watch on the whole existing `ui` directory. It handles new assets but recursively includes source-checkout `ui/node_modules`, causing unrelated dependency churn and a larger scan. The scoped option avoids that subtree when source/public/dist directories exist. Published layouts lack node_modules. Do not watch the workspace root or its target.

## Qualification and evidence

Baseline recipe, both runs on the same host, toolchain, source, target, and configuration:

```sh
CARGO_LOG=cargo::core::compiler::fingerprint=info scripts/dev/cargo check --locked -p mockforge-ui --lib --message-format=json
```

Both ordinary admissions used policy `5de45082645bb5f7236dc78f2c41fdd6ee4dcabe4c199be8abe34273f18bad7e`, 12,288 MiB, 200% CPU, and two jobs. The first refreshed the large default UI dependency closure. The second waited 547.214 seconds for admission, then executed for 17.397 seconds. These are diagnostics, not a representative timing comparison or speedup claim. Native checks alone do not establish UI runtime behavior.

The [lossless baseline archive](baseline-raw.tar.xz) contains both raw JSON/stderr pairs, source/configuration identities, timestamp snapshot, and reproduction summary. Size 84040 bytes; SHA-256 `4906604e63cce820dc7101c87058e07e08b673629db166921862c9cdcbe3c6db`. Tracked source remained clean throughout.

An existing remote checkout was inspected using trusted access. It lacked warmed UI artifacts and had constrained temporary storage, so the initial reproduction reused the local target. Offload requires an independently owned target and verified capacity. No host policy, key, or global configuration changed.

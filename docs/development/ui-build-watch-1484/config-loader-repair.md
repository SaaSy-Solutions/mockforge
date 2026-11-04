# Avoid temporary config writes during Cargo-owned frontend builds

The authentic failure fixture seeded successful owned HTML, then introduced invalid JavaScript. Vite failed while the old owned index remained present. The copied production build script correctly selected the placeholder, and the native test passed. The unchanged repeat also embedded the placeholder correctly, but kept only 33 of 34 artifacts fresh and advanced the build-script output timestamp. Cargo fingerprint diagnostics identify the watched UI parent as changed.

[Vite 6 documents](https://v6.vite.dev/config/#config-intellisense) that its default config bundler writes a temporary config under `node_modules/.vite-temp`. An active absent prebuilt fallback requires an existing-parent watch, so that producer write caused another loop. The documented `--configLoader runner` loads the ESM configuration without a temporary bundled file. External CJS packages remain supported.

Only the validated Cargo-owned output branch now passes that flag. The standalone no-argument command retains its original loader and output behavior. Source and fallback watches stay intact; no file timestamps, compiler profiles, admission settings, or global configuration are changed. The rendering and version bodies remain unchanged.

The [lossless failed-repeat archive](real-failure-loop-raw.tar.xz) is 19492 bytes with SHA-256 `bb42002fef8f1d596779ba92b66f46df6de8a1b8f73c624fa697eb7729e7afd1`. All three Cargo commands exited zero and ran their one native test; the functional qualifier failed because the unchanged repeat rebuilt. That is a real freshness regression, not a Rust compilation failure or a failed fallback selection.

The final flag path still requires authentic success/default-output equivalence, a real failure retaining stale owned output, unchanged fallback freshness, expanded input controls, and the full-package fallback check pair. Full application frontend behavior remains unqualified if its existing source build cannot complete.

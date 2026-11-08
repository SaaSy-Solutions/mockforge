# Keep frontend output outside watched source inputs

The intermediate fallback-only repeat was fresh, but the actual frontend hook had failed. A controlled successful producer then wrote identical bytes into a watched distribution file on every invocation. Six native input controls passed; the unchanged Cargo repeat rebuilt anyway. Its build-script output timestamp advanced. The raw results establish an additional loop, not a successful frontend result.

The first attempt failed before native tests because the other host's existing `lld` selection could not find standard C libraries. The approved correctness retry used only command-local `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/usr/bin/cc`. Both receipts and compiler-environment identities remain in the archive. This is not a cross-host timing comparison. Neither global configuration nor admission policy changed.

## Selected output boundary

Cargo says build scripts should modify only [`OUT_DIR`](https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script). Build the frontend into an explicit Cargo-owned output directory. Embed that directory only after a successful producer with an index file; otherwise retain the existing prebuilt `ui/dist` or placeholder selection. Keep external fallback assets read-only. Watch their appearance and mutations only while they are relevant fallback inputs. Source, public assets, and the nine production configuration/index/lock inputs remain watched. Successful source builds must not watch owned output or node_modules.

Give `build_ui.sh` an optional output argument, preserving no-argument standalone behavior. Verify real pinned pnpm forwarding without a stray `--`. The current pinned package script is `vite build`; also cover shell-chain forwarding. Route the Vite visualizer's independently hardcoded `./dist/stats.html` into the same owned output directory. Empty only that explicit owned directory through Vite's output contract. Preserve version and rendering bodies except necessary selected-path plumbing.

Content-aware timestamp restoration is the rejected alternative. It requires recursive byte/metadata comparison, has portability risks, and retains writes into watched inputs. Removing fallback watches would hide future prebuilt assets. Creating fake assets would alter fallback behavior. None is needed when output has its own owner.

New controls must preserve the old writing-into-input failure, then model the corrected output path. Test successful production or an authentic minimal Vite build, unchanged repeats, source config/index mutations, public/prebuilt appearance and removal, and packaged/fallback behavior. The full production pair remains explicitly fallback-only until a successful frontend is actually exercised. Optional root-hook and environment appearance are the previously documented limits.

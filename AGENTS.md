# MockForge agent development

Read [CLAUDE.md](CLAUDE.md) for architecture, conventions, security requirements,
and existing validation rules. Use [the local Rust workflow](docs/development/RUST_BUILD_WORKFLOW.md)
for all ordinary compilation, regardless of agent provider:

```bash
scripts/dev/cargo check -p <affected-package>
scripts/dev/cargo test -p <affected-package>
scripts/dev/cargo clippy -p <affected-package> --all-targets -- -D warnings
```

Use focused compiler feedback, then relevant tests, feature/runtime/integration
validation, and required CI. A successful check alone is not completion. Preserve
registry tenant isolation, authentication, authorization, source-to-executable
verification, publishing guards, and existing quality gates. Shared contracts and
workspace changes require downstream validation. Do not clean caches, update the
lockfile, launch release builds, or run full-workspace checks after ordinary leaf
edits. Run one focused watcher; never share mutable targets across worktrees.

The launcher shares existing host admission slots with other participating
repositories; do not substitute another coordinator or silently bypass admission.
No global Cargo interception or shell provisioning is required.

The measured native default-feature library recipe is
`scripts/dev/cargo check -p mockforge-platform-signing --lib`, after compatible
successful warm-target validation. Optional AWS features and broader commands
remain ordinary. Warm receipt eligibility never replaces focused tests.

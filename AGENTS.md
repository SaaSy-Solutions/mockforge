# Repository Guidelines

MockForge is a multi-crate Rust workspace for building, simulating, and deploying multi-protocol mocks. Follow the practices below to stay aligned with the project's tooling and workflow expectations.

## Project Structure & Module Organization
- `crates/`: individual protocol, plugin, and observability crates; prefer adding new functionality inside an existing domain crate before creating a new one.
- `tests/` and `fixtures/`: integration suites and reusable scenario assets; reference fixtures via relative paths to keep cross-crate tests portable.
- `examples/`, `demos/`, and `docs/`: runnable showcases, UI demos, and mdBook sources; keep samples minimal and update the book when APIs change.
- `scripts/`, `deploy/`, `helm/`, and `docker-compose*.yml`: automation for CI/CD and operations; mirror any new script usage in `Makefile` targets.

## Build, Test, and Development Commands
- `make build` / `make build-release`: compile the full workspace in debug or release mode.
- `make test`, `make test-coverage`, `make bench`: run unit/integration tests, coverage via `cargo llvm-cov`, and Criterion benchmarks respectively.
- `make dev`: launch the watch loop (`cargo watch`) for check, test, and clippy; use `./scripts/dev.sh` for coordinating the Rust backend with the UI.
- `cargo run -p mockforge-cli -- serve ...`: start the CLI against an OpenAPI spec; see `examples/openapi-demo.json` for a baseline invocation.

## Coding Style & Naming Conventions
- Format with `make fmt` (4-space indentation, 100-column width) and gate diffs with `make fmt-check` and `make clippy`.
- Use `snake_case` for modules and functions, `PascalCase` for types, and keep enums exhaustive with Rustdoc comments when exposing public APIs.
- Enable lint enforcement via workspace settings; add targeted `#[allow]` only when referenced in the PR rationale.

## Testing Guidelines
- Write fast unit tests near source crates and protocol-level integration tests in `tests/`; mock data should live in `fixtures/`.
- Use `make test` locally, `make test-coverage` before merging significant changes, and `make test-mutants` for core protocol or plugin logic.
- Name test modules after the feature under test (e.g., `mod graphql_schema_tests`) and prefer `async` tests when interacting with Tokio components.

## Commit & Pull Request Guidelines
- Follow the imperative style evident in history (e.g., `Add path dependencies to remaining crates`); scope each commit to one logical change.
- Reference issues in the body (`Refs #123`) and note config toggles or migrations in bullet form.
- Pre-flight every PR with `make check-all`, summarize validation steps, and include screenshots or logs when touching the UI (`crates/mockforge-ui`).

## Configuration & Security Tips
- Start from `config.template.yaml` or `config.example.yaml`; never commit secrets and prefer environment overrides for credentials.
- Consult `SECURITY.md` and `config.example.auth.yaml` when adjusting auth or encryption defaults, and run `make audit` after dependency updates.
- Helm, Docker, and Jenkins assets should stay versioned with matching image tags—update `docker-compose*.yml` and `helm/values.yaml` together.

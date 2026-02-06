# MockForge — Claude Code Guide

## Project Overview

MockForge is a high-performance mock API server written in Rust. It's a workspace of 42 crates supporting 10+ protocols (HTTP, gRPC, WebSocket, GraphQL, Kafka, MQTT, AMQP, SMTP, FTP, TCP), with an admin UI (React 19 + TypeScript), plugin system, benchmarking (k6), and a multi-tenant registry server.

- **Language**: Rust 2021 edition, `unsafe_code = "deny"` workspace-wide
- **Version**: 0.3.31+ (workspace.package.version in root Cargo.toml)
- **License**: MIT OR Apache-2.0
- **Repository**: https://github.com/SaaSy-Solutions/mockforge

## Quick Start

```bash
# Build
cargo build --workspace

# Run with example spec (HTTP :3000, WS :3001, gRPC :50051, Admin UI :9080)
make run-example

# Run tests
cargo test --workspace

# UI dev server
cd crates/mockforge-ui/ui && pnpm install && pnpm dev
```

## Running MockForge

The main binary is `mockforge-cli`. Key environment variables and flags:

```bash
# Minimal: serve an OpenAPI spec on HTTP
cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --http-port 3000

# Full-featured: all protocols + admin UI
MOCKFORGE_LATENCY_ENABLED=true \
MOCKFORGE_FAILURES_ENABLED=false \
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
cargo run -p mockforge-cli -- serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 --ws-port 3001 --grpc-port 50051 \
  --admin --admin-port 9080
```

The admin UI is available at `http://localhost:9080` when `--admin` is passed.

## Architecture

Request flow: **OpenAPI spec** → parsed by `mockforge-core` → routes registered → incoming request matched by path/method → response generated (with optional template expansion, latency injection, chaos engineering) → returned to client.

Key architectural boundaries:
- `mockforge-core` owns routing, middleware, and OpenAPI parsing — most protocol crates depend on it
- Each protocol crate (`mockforge-http`, `mockforge-grpc`, etc.) implements its own server/listener
- `mockforge-plugin-core` defines the plugin trait; `mockforge-plugin-loader` dynamically loads `.so`/`.dylib` plugins
- `mockforge-bench` generates k6 scripts from OpenAPI specs via Handlebars templates

## Project Structure

### Foundation Layer
| Crate | Purpose |
|-------|---------|
| `mockforge-core` | Core server, routing, OpenAPI parsing, middleware |
| `mockforge-data` | Data models, serialization, storage |
| `mockforge-template-expansion` | Handlebars template rendering |
| `mockforge-schema` | Schema validation |
| `mockforge-world-state` | Stateful mock behavior |

### Protocol Layer
| Crate | Purpose |
|-------|---------|
| `mockforge-http` | HTTP protocol (primary protocol) |
| `mockforge-grpc` | gRPC protocol support |
| `mockforge-ws` | WebSocket protocol |
| `mockforge-graphql` | GraphQL protocol |
| `mockforge-kafka` | Kafka protocol |
| `mockforge-mqtt` | MQTT protocol |
| `mockforge-amqp` | AMQP protocol |
| `mockforge-smtp` | SMTP protocol |
| `mockforge-ftp` | FTP protocol |
| `mockforge-tcp` | TCP protocol |

### Plugin & Extension Layer
| Crate | Purpose |
|-------|---------|
| `mockforge-plugin-core` | Plugin trait definitions |
| `mockforge-plugin-sdk` | SDK for plugin authors |
| `mockforge-plugin-loader` | Dynamic plugin loading |
| `mockforge-plugin-registry` | Plugin marketplace |
| `mockforge-plugin-cli` | Plugin CLI management |

### Bench, Test & Observability
| Crate | Purpose |
|-------|---------|
| `mockforge-bench` | k6 script generation, WAFBench, OWASP testing |
| `mockforge-test` | Test framework and utilities |
| `mockforge-observability` | Observability infrastructure |
| `mockforge-tracing` | Distributed tracing |
| `mockforge-analytics` | Usage analytics |
| `mockforge-reporting` | Report generation |
| `mockforge-chaos` | Chaos engineering |
| `mockforge-performance` | Performance profiling |
| `mockforge-pipelines` | Request/response pipelines |
| `mockforge-recorder` | Traffic recording |
| `mockforge-route-chaos` | Per-route chaos injection |
| `mockforge-scenarios` | Test scenario management |

### User-Facing
| Crate | Purpose |
|-------|---------|
| `mockforge-cli` | Main CLI binary |
| `mockforge-ui` | React admin UI (Vite + React 19 + TypeScript) |
| `mockforge-sdk` | Client SDK |
| `mockforge-registry-server` | Multi-tenant registry API |

### Infrastructure
| Crate | Purpose |
|-------|---------|
| `mockforge-tunnel` | Tunnel support |
| `mockforge-vbr` | Virtual branch resilience |
| `mockforge-collab` | Collaborative features (SQLite/SQLx) |
| `mockforge-k8s-operator` | Kubernetes operator |
| `mockforge-runtime-daemon` | Runtime daemon service |
| `mockforge-federation` | Federation support |

### Examples & Tests
| Crate | Purpose |
|-------|---------|
| `examples/plugins/response-graphql` | Example GraphQL plugin |
| `examples/test-integration` | Integration test examples |
| `test_openapi_demo` | OpenAPI demo test fixture |
| `desktop-app` | Desktop application (Tauri) |
| `tests` | Workspace-level integration tests |

## Key Commands

### Rust
```bash
cargo build --workspace                    # Build all
cargo test --workspace                     # Test all
cargo test -p <crate-name>                # Test specific crate
cargo fmt --all --check                    # Check formatting
cargo clippy -p <crate> --all-targets -- -D warnings  # Lint specific crate
cargo clippy --all-targets --all-features -- -D warnings  # Lint all
cargo audit                                # Security audit
cargo deny check licenses sources bans     # License/dependency check
```

### UI (from `crates/mockforge-ui/ui/`)
```bash
pnpm install          # Install deps
pnpm dev              # Dev server
pnpm build            # Production build
pnpm type-check       # TypeScript check (tsc --noEmit)
pnpm lint             # ESLint
pnpm test             # Vitest
pnpm test:e2e         # Playwright E2E
```

### Make Targets
```bash
make build            # cargo build --workspace
make test             # cargo test --workspace
make clippy           # Lint all
make fmt              # Format all
make check-all        # fmt + clippy + audit + deny + test
make dev              # Watch mode (check + test + clippy)
make bench            # Performance benchmarks
make test-coverage    # Coverage with llvm-cov
make spellcheck       # Typos check
make test-mutants     # Mutation testing
make sqlx-prepare     # Regenerate SQLx query cache (mockforge-collab)
```

## Development Guidelines

### Rust Conventions
- **`unsafe` is denied** workspace-wide. Any exception requires `// SAFETY:` comment explaining why it's sound
- **Error handling**: `thiserror` for library crates, `anyhow` only in binaries and tests
- **Logging**: Use `tracing` macros (`tracing::info!`, `tracing::error!`), never `println!` or `eprintln!`
- **Async**: Tokio runtime. Use `#[tokio::test]` for async tests
- **Dependencies**: Shared deps defined in workspace `[dependencies]` — use `dep.workspace = true` in crate Cargo.toml

### Template Variable Consistency (Critical — Issue #79)
When working with Handlebars templates (`.hbs` files) and their Rust rendering code:
1. Every `{{variable}}` and `{{#if flag}}` in a template MUST be present in ALL code paths that build template data
2. The k6 bench template (`k6_script.hbs`) is rendered by 3 different code paths in `command.rs` — changes must be verified in all 3
3. Run `/template-check` after any template or template-data changes

### Common Pitfalls
- **Header parsing**: `parse_headers()` splits on commas then colons. Cookie values with semicolons work fine, but Cookie values containing commas will break the parser
- **k6 script generation**: Functions may be defined but never called if the `{{#if flag}}` controlling the call site evaluates to false due to a missing template variable (the issue #79 pattern)
- **SQLx**: `mockforge-collab` uses SQLx with offline mode. After changing queries, run `make sqlx-prepare` to regenerate the query cache
- **Workspace deps**: Adding a dependency to the workspace `Cargo.toml` affects compile time for all crates that inherit it

### UI Conventions
- TypeScript strict mode, no `any` types
- Package manager: **pnpm** (not npm/yarn)
- Build: Vite 6, React 19, Tailwind CSS 4
- Components: Material UI 7 + Radix UI primitives
- State: Zustand 5 + TanStack React Query 5
- Routing: React Router DOM 7

### Git Conventions
Commit format: `<type>(<scope>): <description> (#issue)`
- Types: `feat`, `fix`, `chore`, `test`, `docs`, `refactor`, `perf`
- Scope: crate name or area (e.g., `bench`, `core`, `ui`, `cli`)
- Example: `fix(bench): security payloads now injected + cookie dedup in all templates (#79)`
- Use `[skip ci]` for non-code changes (baselines, docs)

## Testing Strategy

- **Unit tests**: Inline `#[cfg(test)]` modules in each source file
- **Integration tests**: `tests/` directory at workspace root, plus per-crate `tests/` dirs
- **UI unit tests**: Vitest (`pnpm test`)
- **UI E2E tests**: Playwright (`pnpm test:e2e`)
- **Benchmarks**: Criterion.rs (`cargo bench`)
- **Mutation testing**: `cargo mutants` (`make test-mutants`)

## Claude Code Configuration

This project uses:
- **Rules** (`.claude/rules/`): Behavioral procedures loaded every session
- **Agents** (`.claude/agents/`): Specialized subagents for code review, testing, etc.
- **Skills** (`.claude/skills/`): Invocable commands (`/verify`, `/template-check`, etc.)
- **Commands** (`.claude/commands/`): Git workflow commands (`/commit`, `/commit-push-pr`)
- **Hooks** (`.claude/hooks/`): Hookify rule engine for warnings/blocks
- **MCP** (`.mcp.json`): Playwright browser automation for UI verification

### Key Skills
| Skill | Purpose |
|-------|---------|
| `/verify [scope]` | Run self-verification checklist |
| `/template-check [name]` | Cross-reference template variables with code |
| `/code-review [target]` | Launch parallel review agents |
| `/bench-review [spec]` | Validate benchmark scripts |
| `/crate-explore <crate>` | Deep-dive into a crate's structure |
| `/commit` | Smart commit with auto-generated message |
| `/commit-push-pr` | Full branch + commit + push + PR workflow |
| `/hookify <description>` | Create hook rules from plain English |
| `/hookify:list` | List all hookify rules |
| `/hookify:configure` | Enable/disable/delete hookify rules |

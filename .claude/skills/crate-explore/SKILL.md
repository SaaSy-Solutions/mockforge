---
user-invocable: true
allowed-tools: [Read, Glob, Grep, Task]
description: Deep-dive into a crate's structure, public API, and dependencies
argument-hint: "<crate-name>"
---

# /crate-explore — Crate Deep Dive

Explore a MockForge crate's Cargo.toml, public API, module structure, and dependencies.

## Process

### 1. Locate the Crate

Find the crate at `crates/<crate-name>/`. If not found, search for partial matches.

### 2. Read Cargo.toml

Extract:
- Dependencies (workspace and external)
- Features
- Build dependencies
- Dev dependencies

### 3. Map Module Structure

Read `src/lib.rs` (or `src/main.rs` for binaries):
- List all `mod` declarations
- List all `pub use` re-exports
- Identify the public API surface

### 4. Analyze Public API

For each public module:
- List public structs, enums, traits, functions
- Note key trait implementations
- Identify the most important types

### 5. Map Dependencies

- **Internal**: Which workspace crates does this depend on?
- **External**: Key external dependencies
- **Reverse**: Which workspace crates depend on THIS crate?

### 6. Launch Code Explorer (if complex)

For crates with complex trait hierarchies or cross-crate interactions, launch the `code-explorer` agent for deeper analysis.

### 7. Output

```
## Crate: <crate-name>

### Overview
<One-line description from Cargo.toml>

### Module Structure
```
src/
├── lib.rs (re-exports: TypeA, TypeB, trait_c)
├── module_a.rs (pub structs: Foo, Bar)
├── module_b.rs (pub fn: process, validate)
└── internal/ (private implementation)
```

### Public API
- `struct TypeA` — description
- `trait TraitB` — description
- `fn function_c()` — description

### Dependencies
- Internal: mockforge-core, mockforge-data
- Key external: serde, tokio, axum

### Dependents
- mockforge-cli, mockforge-testing
```

## Rules

- Always start with Cargo.toml and lib.rs
- Focus on public API — don't enumerate private internals unless asked
- Include file:line references for key items

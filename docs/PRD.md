# MockForge PRD

Last updated: 2025-09-09 01:33:11Z

## Summary

Mocking platform for HTTP, WS, and gRPC with record/replay, proxy, overrides, latency/failure injection, and data synthesis (RAG+Faker).

## Problem & Goals

Teams need to develop and test frontends/services before real backends exist, and they need realistic, evolving data that reflects relationships across resources. Existing tools mock HTTP well, some mock WS, but few generate relationship-aware data. MockForge provides:

- **Spec-driven mocks (OpenAPI v3)** for HTTP + WebSockets
- **Behavior controls** (latency, conditional responses, proxy/record-replay)
- **RAG-powered data synthesis** that understands schemas + relationships to output "realish" data

**Primary users:** frontend devs, integration testers, API designers

**Non-goals for MVP:** full admin auth, multi-tenant SaaS, advanced UI performance analytics

## Users

- Frontend engineers
- Backend engineers
- QA/Automation

## Success Criteria (MVP)

- **Start server from CLI** with an OpenAPI file; all paths auto-mocked
- **At least one WebSocket route** with message matching & scripted responses
- **Per-tag latency profiles** applied to responses
- **Proxy prefix with upstream targeting**; record fixtures for GET; replay from files
- **Data generator:**
  - Understands OpenAPI components.schemas, builds an entity graph (refs, arrays, foreign-key-like fields)
  - Generates consistent, relationally coherent sample datasets
  - Optional RAG mode: uses model(s) + embeddings to produce realistic categorical/textual fields per domain hints
- **CI runs smoke tests** against the mock and WS flows

## Features (MVP)

### A. Spec Ingestion & Routing

- Import OpenAPI v3 (JSON/YAML)
- Route table created for all operations; path params {id} → :id
- Choose response: prefer examples → synthesize from schema
- Content negotiation (default application/json)
- Request/response validation against schema (warn/block configurable)

### B. HTTP Features

- Conditional responses via rules (method, headers, query, JSONPath on body)
- Latency profiles per OpenAPI tag (auth fast, analytics slower, etc.)
- Fault injection (fixed status overrides; error bodies)
- Proxy mode under a prefix (e.g., /proxy/*) with env-configurable upstream

### C. WebSockets

- Route definitions with:
  - on_connect messages
  - on_message rules (JSONPath matcher) → responses
  - Simple streaming timers (broadcast or per-connection)
- Minimal DSL for WS behavior (declarative YAML)

### D. Record & Replay

- Proxy GETs to upstream and save fixtures (JSON files per route)
- When a fixture exists, serve it before synthetic/mock responses
- Lightweight cassette format (JSONL) for future write-ops (post-MVP)

### E. RAG-powered Synthetic Data (unique feature)

- Build entity graph from OpenAPI schemas (detect refs, arrays, key-looking fields)
- Seeded generator: deterministic by seed; pluggable faker rules (names, emails, addresses, images)
- RAG mode (optional):
  - Compute embeddings for schema & field hints (descriptions, enums)
  - Use a local or configured LLM to generate exemplar rows conditioned on:
    - field types, enums, constraints
    - relationships (parents/children cardinalities)
    - domain hints (from schema description, x-* vendor extensions)
  - Cache generated corpora; hydrate endpoints from them
- Output dataset packs (NDJSON/Parquet/CSV) to share across environments

### F. CLI & Config

- Single binary CLI:
  - `mockforge serve --spec openapi.yaml --port 3000`
  - `mockforge record --upstream https://api.example.com --out fixtures/`
  - `mockforge replay --fixtures fixtures/`
  - `mockforge data gen --spec openapi.yaml --out data/ --seed 42 --rag`
  - `mockforge validate --spec openapi.yaml --mocks ./mocks`
- Config file mockforge.yaml for ports, profiles, proxy, data settings

### G. UI (minimal MVP)

- Static status page at /__admin:
  - Routes list, recent requests, latency profile indicators
  - Toggle fixture priority per route
  - Tail logs (last N)
- No auth in MVP; dev-only

### H. Observability

- tracing logs with span IDs
- Prometheus /metrics: request counts, p50/p95 latency, WS connections
- Request log to JSONL for quick debugging

# Technical Design Document — MockForge MVP

## 1. Overview

MockForge is a Rust-based, open-source platform for mocking APIs across HTTP, WebSocket, and gRPC protocols. It enables frontend and integration development without a live backend, simulates realistic latency/failure conditions, and generates relationship-aware synthetic datasets using a combination of faker-like primitives and RAG (retrieval-augmented generation).

The system is designed as a modular workspace of crates that share a core engine for request routing, schema validation, and data generation.

## 2. Architecture

### High-Level Diagram

```text
             +------------------+
             |     CLI / UI     |
             +--------+---------+
                      |
         +------------+------------+
         |    Core Engine (axum)   |
         +------------+------------+
                      |
   +----------+-------+---------+-----------+
   |          |                 |           |
 HTTP Mock  WS Mock          gRPC Mock   Data Gen
(axum)    (tokio-ws)         (tonic)     (faker+RAG)
```

### Crate Structure

```bash
mockforge/
  crates/
    mockforge-core/   # routing, validation, latency, proxy
    mockforge-http/   # HTTP mock implementation
    mockforge-ws/     # WebSocket DSL + engine
    mockforge-grpc/   # gRPC server mock
    mockforge-data/   # faker + RAG synthetic data engine
    mockforge-cli/    # CLI frontend
    mockforge-ui/     # /__mockforge admin interface
```

## 3. Modules & Responsibilities

### 3.1 Core Engine (mockforge-core)

Parse OpenAPI and Protobuf descriptors into a unified route registry.

Apply request/response validation:

- **OpenAPI:** via jsonschema crate
- **Protobuf:** via generated prost descriptors

Latency/fault injection:

- Fixed per-tag latency (auth, analytics, etc.)
- Configurable error responses

Proxy mode:

- Forward HTTP requests under /proxy/*
- Forward gRPC unary calls when upstream configured

Fixture/record-replay management (GET for HTTP, unary for gRPC).

### 3.2 HTTP Mock (mockforge-http)

Uses axum router.

Endpoints:

- Prefer examples from spec → synthetic schema-driven response
- Conditional responses (headers, query, JSONPath match)

Fixtures:

- If file exists in fixtures/, serve before synthetic/proxy

### 3.3 WebSocket Mock (mockforge-ws)

Based on tokio-tungstenite integrated into axum.

DSL in YAML/JSON:

- **onConnect:** list of messages
- **onMessage:** match rules → responses
- **stream:** timed broadcasts or per-client pushes

Data templating via Handlebars + faker helpers (uuid, randInt, etc.).

### 3.4 gRPC Mock (mockforge-grpc)

Uses tonic for gRPC server.

Load .proto descriptors (via prost-build or reflection).

Auto-generate stub services:

- **Unary** → synthesize response payload
- **Server streaming** → stream N responses or dataset rows
- **Bidirectional streaming** → echo or scripted responses

Fixtures for unary calls (optional in MVP).

### 3.5 Data Engine (mockforge-data)

Schema graph builder:

- Parse OpenAPI schemas + Protobuf messages
- Identify entities, keys, references (e.g., userId → User)

Faker mappers:

- string(format=email) → random email
- date-time → ISO timestamps
- enum → pick from values

Relationship hydrator:

- Generate parents before children
- Ensure referential integrity

RAG mode:

- Embed schema docs + field descriptions
- Query LLM to generate rows consistent with schema + relationships
- Validate output with schema; repair if invalid

### 3.6 CLI (mockforge-cli)

Commands:

```bash
mockforge serve --spec openapi.yaml --proto service.proto --port 3000
mockforge record --out fixtures/ --upstream http://real.api
mockforge replay --fixtures fixtures/
mockforge data gen --spec openapi.yaml --proto service.proto --rows 1000 --out datasets/ --rag
mockforge validate --spec openapi.yaml --fixtures fixtures/
```

### 3.7 UI (mockforge-ui)

`/__mockforge` admin interface (served by axum):

- Route list (HTTP, WS, gRPC)
- Fixture presence indicator
- Latency profile per route
- Recent request log
- Metrics endpoint (/metrics) with Prometheus format

## 4. Data Flow

### 4.1 HTTP Request

```text
[Client] → [Axum Router] → [Route Lookup]
         → [Fixture?] → [Synthetic Data Gen?] → [Proxy?]
         → [Response with latency/fault injection]
```

### 4.2 WebSocket

```text
[Client] → [WS Upgrade] → [onConnect messages]
         → [Incoming msg → DSL match → responses/streams]
```

### 4.3 gRPC

```text
[Client] → [tonic Service Registry]
         → [Method stub]
         → [Fixture?] → [Synthetic Data Gen?] → [Proxy?]
         → [Unary response / Stream]
```

### 4.4 RAG Data Generation

```text
[Spec schemas] → [Entity Graph Builder]
               → [Embed descriptions + fields]
               → [Vector DB store]
               → [Prompt LLM for sample rows]
               → [Validate rows against schema]
               → [Cache dataset]
               → [Serve as API responses]
```

## 5. External Tooling

### Rust Ecosystem

- **axum** (HTTP routing)
- **tonic + prost** (gRPC)
- **tokio-tungstenite** (WebSockets)
- **handlebars** (templating)
- **jsonschema** (validation)
- **clap** (CLI)
- **tracing, metrics, prometheus** (observability)

### Data/RAG Tooling

- **Embeddings:** ONNX models (all-MiniLM-L6-v2.onnx) via ort
- **Vector store:** SQLite + sqlite-vss (or tantivy)
- **LLM backend:** pluggable (local llama-rs or remote HTTP)

## 6. Milestones

### M0: Skeleton crates, CLI scaffold, basic HTTP mocking

### M1: Add WebSocket DSL + streaming

### M2: gRPC unary + streaming mocks

### M3: Fixtures + proxy/record/replay

### M4: Data engine (faker + RAG integration)

### M5: Admin UI + observability

## 7. Risks & Mitigation

- **RAG outputs invalid JSON** → enforce schema validation + auto-repair
- **gRPC descriptor parsing complexity** → start with prost-compiled stubs, expand to dynamic reflection later
- **WS state complexity** → keep DSL declarative; add plugin hooks post-MVP
- **Performance** → use async (tokio) everywhere, cache datasets in memory

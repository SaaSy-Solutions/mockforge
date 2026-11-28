# CLI Reference

MockForge provides a comprehensive command-line interface for managing mock servers and generating test data. This reference covers all available commands, options, and usage patterns.

## Global Options

All MockForge commands support the following global options:

```bash
mockforge-cli [OPTIONS] <COMMAND>
```

### Global Options

- `-h, --help`: Display help information

## Commands

### `proxy` - Forward Traffic and Record Contract Violations

```bash
mockforge proxy --upstream http://127.0.0.1:8080 --port 9081 \
  --spec api.yaml --validate-conformance
```

The proxy listens on `127.0.0.1:9081` by default. `--host` and `--port`
override the listener; `--upstream` (alias `--target`) selects the upstream.
The spec can be JSON or YAML. `MOCKFORGE_PROXY_SPEC` and
`MOCKFORGE_PROXY_VALIDATE_CONFORMANCE=true` are the environment equivalents.
Invalid or missing specs fail startup when validation is enabled.

Validation is observational: invalid requests still reach the upstream, and
upstream status codes, redirect responses, and response bytes are preserved.
Inspect the shared conformance buffer at
`GET /__mockforge/api/conformance/violations?limit=50` on the proxy listener;
`DELETE` on that path clears it. These routes are enabled with validation.
The usual `MOCKFORGE_CONFORMANCE_BUFFER_SIZE` and
`MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE` settings govern the buffer.

`--validate-conformance-strict` also enables validation and rejects invalid
requests before forwarding them. An upstream response that violates its
declared contract produces a `502` response. Set
`MOCKFORGE_PROXY_VALIDATE_CONFORMANCE_STRICT=true` for the same policy.
Ctrl-C and SIGTERM stop the listener gracefully.


Conformance response-schema checks currently cover declared JSON schemas.
Local response references, exact status codes, status ranges, and defaults
are supported. Schema selection uses the upstream `Content-Type`, including
JSON suffix types such as `application/problem+json` and declared media
wildcards. Other declared formats pass through without JSON schema checks;
missing or undeclared response media types are violations. HEAD, 204 and 304
responses have no representation body to validate. Paths absent from the
spec pass through without validation.

Validation decodes gzip, deflate, Brotli, and Zstandard on a separate copy;
forwarded bodies and repeated response headers remain unchanged. Encoded
and decoded validation copies are limited to 16 MiB and four encoding layers.
Invalid, unsupported, or oversized encoded content is reported as a violation.
Passive mode still forwards it; strict mode rejects invalid requests or
returns 502 for invalid responses.

### `serve` - Start Mock Servers

The primary command for starting MockForge's mock servers with support for HTTP, WebSocket, and gRPC protocols.

```bash
mockforge-cli serve [OPTIONS]
```

#### Server Options

**Port Configuration:**
- `--http-port <PORT>`: HTTP server port (default: 3000)
- `--ws-port <PORT>`: WebSocket server port (default: 3001)
- `--grpc-port <PORT>`: gRPC server port (default: 50051)

**API Specification:**
- `--spec <PATH>`: OpenAPI spec file for HTTP server (JSON or YAML format)

**Configuration:**
- `-c, --config <PATH>`: Path to configuration file

#### Admin UI Options

**Admin UI Control:**
- `--admin`: Enable admin UI
- `--admin-port <PORT>`: Admin UI port (default: 9080)
- `--admin-embed`: Force embedding Admin UI under HTTP server
- `--admin-mount-path <PATH>`: Explicit mount path for embedded Admin UI (implies `--admin-embed`)
- `--admin-standalone`: Force standalone Admin UI on separate port (overrides embed)
- `--disable-admin-api`: Disable Admin API endpoints (UI loads but API routes are absent)

#### Validation Options

**Request Validation:**
- `--validation <MODE>`: Request validation mode (default: enforce)
  - `off`: Disable validation
  - `warn`: Log warnings but allow requests
  - `enforce`: Reject invalid requests
- `--aggregate-errors`: Aggregate request validation errors into JSON array
- `--validate-responses`: Validate responses (warn-only)
- `--validation-status <CODE>`: Validation error HTTP status code (default: 400)

#### Response Processing

**Template Expansion:**
- `--response-template-expand`: Expand templating tokens in responses/examples

#### Chaos Engineering

**Latency Simulation:**
- `--latency-enabled`: Enable latency simulation

**Failure Injection:**
- `--failures-enabled`: Enable failure injection

#### Examples

**Basic HTTP Server:**
```bash
mockforge-cli serve --spec examples/openapi-demo.json --http-port 3000
```

**Full Multi-Protocol Setup:**
```bash
mockforge-cli serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --admin-port 9080 \
  --response-template-expand
```

**Development Configuration:**
```bash
mockforge-cli serve \
  --config demo-config.yaml \
  --validation warn \
  --response-template-expand \
  --latency-enabled
```

**Production Configuration:**
```bash
mockforge-cli serve \
  --config production-config.yaml \
  --validation enforce \
  --admin-standalone
```

### `init` - Initialize New Project

Create a new MockForge project with a template configuration file.

```bash
mockforge-cli init [OPTIONS] <NAME>
```

#### Arguments

- `<NAME>`: Project name or directory path
  - Use `.` to initialize in the current directory
  - Use a project name to create a new directory

#### Options

- `--no-examples`: Skip creating example files (only create `mockforge.yaml`)

#### Examples

```bash
# Create a new project in a new directory
mockforge-cli init my-mock-api

# Initialize in the current directory
mockforge-cli init .

# Initialize without examples
mockforge-cli init my-project --no-examples
```

#### What Gets Created

1. **mockforge.yaml**: Main configuration file with:
   - HTTP, WebSocket, gRPC server configurations
   - Admin UI settings
   - Core features (latency, failures, overrides)
   - Observability configuration
   - Data generation settings
   - Logging configuration

2. **examples/** directory (unless `--no-examples`):
   - `openapi.json`: Sample OpenAPI specification
   - Example data files

#### See Also

- [Configuration Files Guide](../configuration/files.md)
- [Complete Config Template](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml)

---

### `config` - Configuration Management

Validate and manage MockForge configuration files.

```bash
mockforge-cli config <SUBCOMMAND>
```

#### Subcommands

##### `validate` - Validate Configuration File

Validate a MockForge configuration file for syntax and structure errors.

```bash
mockforge-cli config validate [OPTIONS]
```

**Options:**
- `--config <PATH>`: Path to config file to validate
  - If omitted, auto-discovers `mockforge.yaml` or `mockforge.yml` in current and parent directories

**What Gets Validated:**
- YAML syntax and structure
- File existence
- HTTP endpoints count
- Request chains count
- Missing sections (warnings)

**Examples:**

```bash
# Validate config in current directory
mockforge-cli config validate

# Validate specific config file
mockforge-cli config validate --config my-config.yaml

# Validate before starting server
mockforge-cli config validate && mockforge-cli serve
```

**Output Example:**
```
🔍 Validating MockForge configuration...
📄 Checking configuration file: mockforge.yaml
✅ Configuration is valid

📊 Summary:
   Found 5 HTTP endpoints
   Found 2 chains

⚠️  Warnings:
   - No WebSocket configuration found
```

**Common Issues:**
- **Invalid YAML syntax**: Fix indentation, quotes, or structure
- **File not found**: Check path or run `mockforge init`
- **Missing sections**: Add HTTP, admin, or other required sections

**Note**: Current validation is basic (syntax, structure, counts). For comprehensive field validation, see the [Configuration Validation Guide](../reference/config-validation.md).

#### See Also

- [Configuration Validation Guide](../reference/config-validation.md)
- [Configuration Schema Reference](../reference/config-schema.md)
- [Troubleshooting Guide](../reference/troubleshooting.md)

---

### `data` - Generate Synthetic Data

Generate synthetic test data using various templates and schemas.

```bash
mockforge-cli data <SUBCOMMAND>
```

#### Subcommands

##### `template` - Generate from Built-in Templates

Generate data using MockForge's built-in data generation templates.

```bash
mockforge-cli data template [OPTIONS]
```

**Options:**
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml, csv)
- `--template <NAME>`: Template name (user, product, order, etc.)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate 10 user records as JSON
mockforge-cli data template --template user --count 10 --format json

# Generate product data to file
mockforge-cli data template --template product --count 50 --output products.json
```

##### `schema` - Generate from JSON Schema

Generate data conforming to a JSON Schema specification.

```bash
mockforge-cli data schema [OPTIONS] <SCHEMA>
```

**Parameters:**
- `<SCHEMA>`: Path to JSON Schema file

**Options:**
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate data from user schema
mockforge-cli data schema --count 5 user-schema.json

# Generate and save to file
mockforge-cli data schema --count 100 --output generated-data.json api-schema.json
```

##### `open-api` - Generate from OpenAPI Spec

Generate mock data based on OpenAPI specification schemas.

```bash
mockforge-cli data open-api [OPTIONS] <SPEC>
```

**Parameters:**
- `<SPEC>`: Path to OpenAPI specification file

**Options:**
- `--endpoint <PATH>`: Specific endpoint to generate data for
- `--method <METHOD>`: HTTP method (get, post, put, delete)
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate data for all endpoints in OpenAPI spec
mockforge-cli data open-api api-spec.yaml

# Generate data for specific endpoint
mockforge-cli data open-api --endpoint /users --method get --count 20 api-spec.yaml

# Generate POST request body data
mockforge-cli data open-api --endpoint /users --method post api-spec.yaml
```

### `admin` - Admin UI Server

Start the Admin UI as a standalone server without the main mock servers.

```bash
mockforge-cli admin [OPTIONS]
```

#### Options

- `--port <PORT>`: Server port (default: 9080)

#### Examples

```bash
# Start admin UI on default port
mockforge-cli admin

# Start admin UI on custom port
mockforge-cli admin --port 9090
```

### `bench` - OpenAPI-driven Load Testing

Generate and run a k6 load test from an OpenAPI spec.

```bash
mockforge bench --spec api.yaml --target http://localhost:3000 [OPTIONS]
```

#### Common Options

- `-s, --spec <PATH>` — OpenAPI spec file (repeatable for multi-spec mode).
- `--spec-dir <DIR>` — Directory of specs (auto-discovered).
- `-t, --target <URL>` — Target server URL.
- `-d, --duration <SECS>` — Run length (default: 60).
- `--vus <N>` — Virtual users (default: 10).
- `--scenario <NAME>` — `constant` | `ramp-up` | `spike` | `stress` | `soak`.
- `--operations <FILTER>` — Comma-separated operation filter (e.g. `"GET /users,POST /orders"`).
- `--auth <HEADER>` — Authorization header value (e.g. `"Bearer $TOKEN"`).
- `--headers <KV>` — Comma-separated extra headers (e.g. `"X-Tenant: a,X-Region: b"`).
- `--threshold-percentile <P>` / `--threshold-ms <N>` — Latency SLO.
- `--max-error-rate <RATE>` — Error-rate SLO (`0.01` = 1%).
- `--generate-only` — Write the k6 script without running it.
- `--script-output <PATH>` — Where to write the generated script.
- `--targets-file <PATH>` — Multi-target mode (one URL per line).
- `--max-concurrency <N>` — Parallel target limit (default: 10).
- `--insecure` — Skip TLS verification (test self-signed certs).
- `--chunked-request-bodies` — Add `Transfer-Encoding: chunked` to every k6 request body. NOTE: k6 runs on Go's `net/http`, which may still send `Content-Length` based on body type. For guaranteed wire chunking use [`bench-chunked`](#bench-chunked---native-chunked-traffic-generator) instead.

See the [Load Testing chapter](../user-guide/load-testing.md) for the full
options surface (security testing, OWASP API top 10, conformance mode,
data-driven testing) and end-to-end examples.

### `bench-chunked` - Native Chunked Traffic Generator

Send guaranteed `Transfer-Encoding: chunked` request bodies via hyper. Useful
when `bench --chunked-request-bodies` falls back to `Content-Length` (Go's
transport heuristics) and you need real chunked traffic on the wire.

```bash
mockforge bench-chunked --target http://localhost:3000/upload [OPTIONS]
```

#### Options

- `--target <URL>` — Target URL. Required unless `--targets-file` is given.
- `--targets-file <FILE>` — Bench every target in the file in parallel (same text/JSON format as `bench --targets-file`, including per-target `auth`, `headers`, `spec`). Mutually exclusive with `--target`.
- `--max-concurrency <N>` — Max targets benched at once with `--targets-file` (default: 10).
- `--method <METHOD>` — `POST` (default), `PUT`, or `PATCH`.
- `-c, --concurrency <N>` — Concurrent workers per target (default: 10).
- `-d, --duration <DURATION>` — Run length, per operation with `--spec` (default: 30s).
- `--chunk-size-bytes <N>` — Bytes per chunk (default: 1024).
- `--total-size-bytes <N>` — Total body size per request (default: 1048576 = 1 MiB).
- `--chunk-interval-ms <MS>` — Sleep between chunks (default: 0 = back-to-back).
- `--header <NAME: VALUE>` — Extra header (repeatable).
- `--insecure` — Skip TLS verification.
- `--rps <N>` — Cap on request starts per second, per target.
- `--cps` — Open a new connection for every request (connections/s = requests/s).
- `--rounds <N>` / `--repeat-until <DURATION>` — Campaign: re-run the whole pass; rounds go to `<output>/round_N/`, stats to `campaign.jsonl` and `round-summaries/`.
- `--keep-rounds <N>` — Campaign only: keep the newest N `round_*` dirs.

Reports req/s, p50/p95/p99 latency, and a status-code distribution at the end.

### `git-watch` - Watch a Git Repo for Spec Changes

Monitor a Git repository for changes to OpenAPI / spec files and automatically
reload the local mock server when they change. Useful for teams that store
specs in a separate repo from their mock config.

```bash
mockforge git-watch https://github.com/user/api-specs \
  --reload-command "mockforge serve --spec"
```

Common options: `--branch`, `--poll-interval`, `--auth-token` (for private
repos), `--cache-dir`, `--spec-paths` (comma-separated globs to watch).

### `contract-sync` - Sync Contracts from a Git Repo

One-shot pull of OpenAPI / contract specs from a Git repository, with an
optional report comparing the new spec against the current mock config.

```bash
mockforge contract-sync https://github.com/user/api-specs \
  --update --output report.md
```

Use `--update` to overwrite local copies; omit it for a dry-run report.
Pairs well with CI — fail the build if the upstream contract drifted from
what the mock returns.

### `validate-fixtures` - Validate Fixture Files

Lint a directory or single file of fixture data against MockForge's schema.
Useful pre-commit guard for repos that check fixtures into source control.

```bash
# Validate every fixture in a directory
mockforge validate-fixtures --dir ./fixtures

# Validate one file
mockforge validate-fixtures --file ./fixtures/users.json

# Verbose mode — show per-fixture details, not just failures
mockforge validate-fixtures --dir ./fixtures --verbose
```

Exit code 0 on clean, non-zero on any validation failure.

### `mockforge-tui` - Terminal Dashboard (separate binary)

Live dashboard in the terminal showing server status, system metrics
(current and lifetime peak CPU / memory / error rate), request stats, and
recent logs. Auto-refreshes every 2 s.

```bash
mockforge-tui
```

Optionally pair with `MOCKFORGE_METRICS_LOG_FILE=path.csv` to also append a
persistent CSV record while the server runs.

### `bench-pipeline` - HTTP Pipelining Load Test

Load-test a target with HTTP pipelining: synthetic request bodies
(json / xml / urlencoded / multipart) are written back-to-back per
connection before any response is read.

```bash
mockforge bench-pipeline --target http://localhost:3000/api \
  --content-type json --body-size 500KB --pipeline-depth 16 \
  --connections 8 --duration 30s
```

#### Options

- `--target <url>` (required): plain-`http://` target URL.
- `--method <method>`: HTTP method; pipelining is only meaningful with bodies, so the default is `POST`.
- `--content-type <flavour>`: `json` | `xml` | `urlencoded` | `multipart` (default `json`).
- `--body-size <size>`: exact body size per request — bare bytes or KB/MB/GB / KiB/MiB/GiB (default `1KB`).
- `--pipeline-depth <n>`: requests in flight per connection before reading responses (default `16`).
- `--connections <n>`: concurrent connections (default `8`).

### `verify-mocks` - Mock Fidelity / Drift Detection

Compare the mocks generated from an OpenAPI spec against real captured
traffic in a recorder database, and report drift: endpoints whose mocks no
longer match how clients actually call them.

```bash
mockforge verify-mocks --spec openapi.json --capture-db recorder.sqlite
```

To compare the OpenAPI definition used to generate an existing mock against
its current source specification, use `--mock-spec`. Both inputs can be supplied
to check definitions and captured traffic together:

```bash
mockforge verify-mocks --spec upstream-current.yaml --mock-spec mock-generated-from.yaml \
  --fail-on-drift --output fidelity.json
```

The JSON report's `definition.routes` includes every method/path with a
`verified` or `drift-detected` status. Its `change` is `added`, `removed`,
`changed`, or `unchanged`, from the mock to the source. Each delta includes a
JSON-pointer `location` and the corresponding `mock` and `source` values.
Changes to nested schema types, properties, required fields, constraints,
parameters, request bodies and responses are reported. Local references are
resolved, including recursive schemas. Unresolved/external references and
excessive expansion fail explicitly. Documentation text and the ordering of
required fields, enum values and parameters do not count as drift.

#### Options

- `--spec <path>` (required): OpenAPI spec the mocks were generated from (3.x natively; Swagger 2.0 is up-converted automatically).
- `--capture-db <path>`: recorder SQLite database holding captured traffic (what `mockforge serve --recorder` writes; the default path is `./mockforge-recordings.db`).
- `--mock-spec <path>`: OpenAPI definition used by the mock, compared against the current source in `--spec`.
- `--fixtures <path>`: HTTP replay fixture JSON file or directory. Checks every selected fixture's route, status and JSON response schema, and reports source operations with no fixture. See the [fixture format and scope](../user-guide/mock-fidelity.md#validate-an-http-replay-fixture-set).

Supply at least one of `--capture-db`, `--mock-spec` or `--fixtures`. They can be combined.
- `--limit <n>`: maximum recent HTTP exchanges to evaluate, newest first (positive integer; default `500`). Other protocols do not consume this limit.
- `--fail-on-drift`: exit with code 1 when any drift is found — use as a CI gate.
- `--output <path>`: write the structured drift report as JSON.
- `--record`: keep this run's result as the project's latest verification, in
  `.mockforge/reports/verify-mocks.json` beside the project's configuration
  file (or in the current directory without one). `mockforge status` reports
  it. A run with no recorded traffic is kept with outcome `no_traffic`.

### `status` - Project State, Read Only

Report a project's MockForge state without starting or writing anything:
which configuration file `serve` would use (discovered from the directory
exactly as `serve` does), the HTTP port and OpenAPI spec it declares, whether
a MockForge server answers on that port, and the last `verify-mocks --record`
result.

```bash
mockforge status
mockforge status --dir ./services/api --format json
```

#### Options

- `--dir <path>`: the project directory (default: the current directory).
- `--format text|json`: `json` prints one document and nothing else on stdout.
- `--timeout-ms <n>`: how long to wait for the server (default `1000`).

The server probe asks the management API (`GET /__mockforge/api/health`), which
a spec's own `/health` route cannot shadow, then reads uptime from
`/__mockforge/api/stats` and the version from `/__mockforge/api/config`. The
probe uses the declared port; a server started with `--http-port` elsewhere is
not found. `0.0.0.0` and `::` are probed on `127.0.0.1`.

The JSON document (`schema_version` 2):

```json
{
  "schema_version": 2,
  "version": "0.3.238",
  "dir": "/abs/project",
  "config": { "path": "/abs/project/mockforge.yaml", "error": null },
  "http": { "enabled": true, "host": "0.0.0.0", "port": 3000, "openapi_spec": "./examples/openapi.json" },
  "admin": { "enabled": true, "port": 9080 },
  "server": {
    "url": "http://127.0.0.1:3000/__mockforge/api/health",
    "reachable": true, "status": "healthy", "version": "0.3.238",
    "uptime_seconds": 12, "error": null
  },
  "verification": {
    "schema_version": 2, "recorded_at": "2026-10-26T10:30:43Z",
    "mockforge_version": "0.3.238", "spec": "examples/openapi.json",
    "capture_db": "captures.db", "outcome": "faithful",
    "exchanges_checked": 8, "endpoints_with_drift": 0, "endpoints_unverified": 0,
    "unmatched_real_paths": 4, "findings": []
  }
}
```

`config` is null when no configuration file exists; `config.error` names why an
existing one cannot be loaded, and then `http`, `admin` and `server` are null.

`server` is null when HTTP is disabled. `verification.outcome` is `faithful`,
`drift`, `unverified` or `no_traffic`. `unverified` means response media could
not be checked, without a proven contract violation. Proven drift takes
precedence when a run contains both. Status and newly recorded verifications
use schema 2 for this additional outcome; schema-1 verification files remain
readable with `endpoints_unverified` defaulting to zero. Fields may be added
within a schema; a change that breaks readers raises `schema_version`.

### `validate-spec` - OpenAPI Validation, Read Only

Validate one local OpenAPI 2.0 or 3.x document (JSON or YAML) with the
validator `generate` uses, without generating, starting or writing anything.
Only a local file is read; a URL is refused, not fetched.

```bash
mockforge validate-spec api.yaml
mockforge validate-spec ./specs/openapi.json --format json
```

#### Options

- `<spec>` (required): the OpenAPI document.
- `--format text|json`: `json` prints one document and nothing else on stdout.

Exit codes: `0` when the document is valid (warnings allowed), `1` when it has
errors, `2` when it could not be read as an OpenAPI document. The JSON document
is printed in every case.

The JSON document (`schema_version` 1):

```json
{
  "schema_version": 1,
  "mockforge_version": "0.3.237",
  "spec": "api.json",
  "openapi": "3.1",
  "valid": false,
  "errors": 1,
  "warnings": 0,
  "findings": [
    {
      "severity": "error", "rule": "MISSING_INFO", "path": "/info",
      "message": "Missing 'info' section in OpenAPI 3.1 spec",
      "suggestion": "Add an 'info' section with 'title' and 'version' fields"
    }
  ]
}
```

`openapi` is `"2.0"`, `"3.0"`, `"3.1"`, or null when the document is not
OpenAPI. A document that could not be validated has one error finding whose
`rule` is `UNREADABLE`, `INVALID_SYNTAX` or `NOT_OPENAPI`. `path` is a JSON
pointer when the validator names the field.

### `mcp-contract` - MCP Contract Pinning

Record what an MCP tool server declares (`pin`), then report every change
since (`check`). See [MCP Contract Pinning](../user-guide/mcp-contract.md) for
the workflow, the signals and what a contract cannot see.

```bash
mockforge mcp-contract pin --url https://mcp.example.com/mcp --output contracts/acme-mcp.json
mockforge mcp-contract check --contract contracts/acme-mcp.json
mockforge mcp-contract check --contract contracts/acme-mcp.json --fail-on high --format json
```

#### Options

`pin`:

- `--url <URL>` (required): the MCP endpoint, `http` or `https`.
- `--output <FILE>` (required): the contract to write. An existing file is
  replaced only with `--force`.
- `--accept-hidden`: pin although the server's text contains characters that
  do not print.

`check`:

- `--contract <FILE>` (required): the pinned contract.
- `--url <URL>`: ask at another address. Defaults to the contract's own.
- `--fail-on info|low|medium|high`: the lowest level of finding that fails.
  Default `low`.
- `--output <FILE>`: also write the JSON report to this file.

Both:

- `--header NAME:VALUE`: a plain header sent with every request. Repeatable.
- `--auth-env NAME:ENV_VAR`: a credential header whose value is read from the
  environment. The value is never written to the contract.
- `--client-name <NAME>`, `--client-version <VERSION>`: how to name yourself
  to the server. Defaults to `mockforge` and its version.
- `--timeout <SECONDS>`: time allowed for each request. Default `30`.
- `--format text|json`: `json` prints one document and nothing else on stdout.

For `check`, anything not given comes from the contract: the address, the
plain headers, the credential's variable and the client identity.

Exit codes: `0` pinned or unchanged, `1` changed since the contract, `2` when
nothing could be compared. The JSON document is printed in every case.

The document `check --format json` prints (`schema_version` 1):

```json
{
  "schema_version": 1,
  "kind": "mockforge-mcp-contract-report",
  "contract": "contracts/acme-mcp.json",
  "outcome": "unchanged",
  "digest_pinned": "sha256:…",
  "digest_now": "sha256:…",
  "findings": [],
  "summary": { "tools_before": 12, "tools_after": 12, "added": 0, "removed": 0, "changed": 0, "high": 0, "medium": 0, "low": 0, "info": 0 }
}
```

`outcome` is `unchanged`, `drifted` or `not_checked`. Each finding carries a
`kind`, a `severity`, the `tool` it is about, a one-line `summary`, the
`changes` that make it up and the `signals` to read closely. The document
`pin --format json` prints has the `kind` `mockforge-mcp-contract-pin` and an
`outcome` of `pinned` or `not_pinned`.

### `agent-redteam` - Red-Team an Agent

Play a hostile MCP tool server (`write` a suite, serve it with
`serve --mcp-mock-catalog`), write down what the agent sends
(`serve --mcp-mock-record`), and give a verdict on what it sent and on what it
answered (`judge --reply`). See
[Red-Team an Agent](../user-guide/agent-redteam.md) for the suites, the rules
and what the judge cannot see.

```bash
mockforge agent-redteam list
mockforge agent-redteam write tool-poisoning
mockforge agent-redteam judge --suite tool-poisoning.json --calls calls.jsonl --reply reply.txt
mockforge agent-redteam judge --suite tool-poisoning.json --calls calls.jsonl --reply reply.txt --format json
mockforge agent-redteam judge --suite tool-poisoning.json --calls calls.jsonl --reply reply.txt --format sarif --output agent.sarif
```

#### Options

`list`:

- `--format text|json`: `json` prints one document and nothing else on stdout.

`write <SUITE>`:

- `SUITE`: a shipped suite's id, or `all` for every shipped suite.
- `--output <FILE>`: where to write the suite (default: `<id>.json`).
- `--output-dir <DIR>`: with `all`, where to write every suite (default:
  `./agent-redteam`).
- `--force`: replace a file that is already there.

`judge`:

- `--suite <FILE>` (required): the suite, as `write` writes it.
- `--calls <FILE>` (required): what `serve --mcp-mock-record` wrote.
- `--reply <FILE>`: what the agent answered the user, saved as text. A suite
  with a rule about the reply is not run without it.
- `--output <FILE>`: also write the report, in the same form, to this file.
- `--format text|json|sarif|junit`: text to read, one JSON document, SARIF
  2.1.0 for GitHub's Security tab, or JUnit XML for a test report.

A finding's `kind` is `required-tool-not-called`, `forbidden-tool-called`,
`argument-matched`, `reply-matched` or `reply-missing`. The SARIF has a rule
for each of the suite's, at the level its findings have, and a result for each
finding at the call's line of the log or at the reply. A run that could not be
judged is a result of its own, at the error level, with the reason.

Exit codes for `judge`: `0` the agent resisted, `1` a rule was broken, `2`
nothing could be judged. The JSON document is printed in every case.

The document `judge --format json` prints has the `kind`
`mockforge-agent-report`. `verdict` is `pass`, `fail` or `not_run`, and
`reason` says why nothing could be judged. Each finding carries a `kind`
(`required-tool-not-called`, `forbidden-tool-called` or `argument-matched`),
the `rule` it broke, the `seq` of the call, a one-line `summary` and the
`evidence`. A rule may carry `cwe` and `owasp`: the weakness in MITRE's list
as its site writes it (`CWE-1427`), and a category of the OWASP Top 10 for LLM
Applications as OWASP writes it (`LLM01:2025`). The report carries them: in
the text after why the rule matters, and in the SARIF among the properties
of the rule and of its findings, with the tags code scanning filters on.


### `range` - A Safe Target for an Agent

Write a shipped range pack's API (`write`), serve it with `serve --spec`, and
give the agent the brief, the credential and the scope the pack names. A pack
is an API with flaws planted in it, each with the request that proves it. Three
packs ship: `shop`, a shop API; `webhooks`, a receiver of the webhook events of
six providers; and `generated`, the app a generator writes. See
[Range: a Safe Target for an Agent](../user-guide/range.md) for what a pack
holds and what the mock cannot model yet.

```bash
mockforge range list
mockforge range write shop
mockforge range write webhooks
mockforge range write all --output-dir range
mockforge range new --har traffic.har --id orders
mockforge range flaws
mockforge range new --har traffic.har --id orders --flaw no-credential --at "POST /v1/orders"
mockforge range new --har search.har --id search --flaw sql-injection:q --at "GET /v1/search"
mockforge range new --har login.har --id login --flaw nosql-injection:body.password --at "POST /v1/login"
mockforge serve --spec shop.json --http-port 8080 --exchange-log exchanges.jsonl
mockforge range judge --pack shop.pack.json --exchanges exchanges.jsonl
mockforge range compare --pack shop.pack.json --run scout=scout.jsonl --run reader=reader.jsonl
mockforge range run --pack shop.pack.json --agent "my-agent --target {url} --report {report}"
```

#### Options

`list`:

- `--format text|json`: `json` prints one document and nothing else on stdout.

`flaws`:

- `--format text|json`: `json` prints the catalogue as one document.

`write <PACK>`:

- `PACK`: a shipped pack's id, or `all` for every shipped pack.
- `--output <FILE>`: where to write the API (default: `<id>.json`). The pack
  is written beside it, as `<id>.pack.json`.
- `--output-dir <DIR>`: with `all`, where to write every pack (default:
  `./range`).
- `--force`: replace files that are already there.

`new`:

- `--har <FILE>`: a recording, as an HTTP Archive.
- `--spec <FILE>`: an OpenAPI document, in JSON or in YAML.
- `--id <ID>`: the pack's id, lowercase words and digits joined by `-`. The
  pack is written as `<id>.pack.json`, and the API beside it as `<id>.json`.
- `--title <TITLE>`: the pack's title (default: the API's host for a
  recording, the spec's title for a spec).
- `--brief <FILE>`: a file holding what to tell the agent.
- `--output <FILE>`: where to write the API (default: `<id>.json`). The pack
  is written beside it.
- `--keep-keys`: keep a key of a known shape as it was recorded. Without it,
  each is replaced by a made-up key of the same shape, and the command says
  how many.
- `--flaw <KIND[:PARAMETER|:body.FIELD]>`: a kind of flaw to plant, by its id
  as `range flaws` lists it. Each is planted at the operation of the `--at`
  of its place. A kind injected through a query parameter takes the
  parameter's name after a colon, `sql-injection:q`; one injected through a
  field of the JSON request body takes `body.` and the field's name,
  `nosql-injection:body.password`. Either is planted as an answer by
  request, given only to a request that carries the injection, and the
  operation must declare the parameter or the field.
- `--at <METHOD PATH>`: where to plant a flaw: a path of the pack. There is
  one for each `--flaw`.
- `--force`: replace files that are already there.

`judge`:

- `--pack <FILE>`: the pack the run was against.
- `--exchanges <FILE>`: the exchange log `serve --exchange-log` wrote.
- `--format text|json|sarif|junit`: `json` prints the judgement as one
  document; `sarif` prints it as SARIF 2.1.0, for GitHub's Security tab and
  the tools that read it; `junit` prints it as JUnit XML, for a test report.
- `--output <FILE>`: also write the judgement to a file, as one JSON document,
  whatever `--format` says.
- `--findings <FILE>`: the agent's report, a JSON array of findings, each
  with the request it says shows it, to check against the log.

`compare`:

- `--pack <FILE>`: the pack every run was against.
- `--run <NAME=FILE>`: a run to compare, as many as there are runs and at
  least two: a name of its own for the run, and the exchange log
  `serve --exchange-log` wrote for it.
- `--format text|json`: `json` prints the comparison as one document, with
  every judgement whole.

`run`:

- `--pack <FILE>`: the pack to run the agent against.
- `--agent <COMMAND>`: the agent, run through a shell. `{url}` is the mock's
  address, `{brief}` the file that holds the brief and `{report}` the file to
  write its findings to; the same three are in the environment as
  `MOCKFORGE_RANGE_URL`, `MOCKFORGE_RANGE_BRIEF` and `MOCKFORGE_RANGE_REPORT`.
  The command runs through a shell, where `range run` was run.
- `--work-dir <DIR>`: where the run's files go (default:
  `./range-run/<pack>-<time>`).
- `--timeout <DURATION>`: how long the agent may take, as `30s`, `15m` or
  `2h` (default: `15m`).
- `--format text|json|sarif|junit`: the form of the judgement printed
  (default: `text`). The run's `judgement.json` is written as ever. What the
  run says of itself goes to the error stream unless the form is `text`, so
  that what it prints is one document.

`list --format json` prints an array with one object per pack: `id`, `title`,
`brief`, `credentials`, `operations` (how many operations the API has),
`scope` (the paths that are in scope), `flaws`, `injections` and `bait`. A
flaw, an injection or a bait may carry `cwe` and `owasp`: the weakness in
MITRE's list as its site writes it (`CWE-862`), and a category of a list of
OWASP's as OWASP writes it (`A01:2021`, `LLM01:2025`). A pack with one that
is not written that way is refused. The text shows the class after how much
the flaw matters.

`serve --exchange-log <FILE>` writes down every request the mock receives and
the answer it gives, one JSON line per exchange, whoever answered it. The log
is opened before the server listens, and a file that is there is replaced. A
header that carries a credential (`Authorization`, `Proxy-Authorization`,
`Cookie`, `Set-Cookie`) is written as `sha256:` and the digest of its value,
never as sent. So is a value that is a key of a known shape, wherever it is
the whole value of a header or a whole JSON string of a body: `AKIA…`,
`ghp_…`, `sk_live_…` and some ninety other shapes. A body is held up to
16 KiB, and what is cut is marked.

`range flaws` lists the kinds of flaw a pack can plant today, each with the
request that proves it, the answer that shows it, its CWE and OWASP
category, and the SemSec rules whose class it is. `range new` plants a
kind at an operation with `--flaw KIND --at "METHOD PATH"`: the operation's
answer is changed as the kind says, and the flaw that proves it is added to
the pack with its class. Eight kinds are injected through the request and
take its name after a colon: six through a query parameter or a field of
the JSON body, `--flaw sql-injection:q` or `--flaw sql-injection:body.q`; a
reflected script through a query parameter alone; and a NoSQL injection
through a field alone, `--flaw nosql-injection:body.password`. The operation
answers the request that carries the injection, and every other request as it
did.

A proving request may name the query string: `query_with` names parameters it
must carry, with their values, `query_contains` names those whose value must
hold a text, and `query_without` names those it must not. It may name the
fields of a JSON body: `body_with` names those whose value must hold a text,
wherever in the body the field is. It may say how many times the request must
have come, with `times`: a limit that is not there is shown by the same
request answered again and again. A kind planted from the catalogue is proved
by the mark its answer is given for, so every request the mock answered weakly
counts and not the one injection the kind was written with. The answer that shows a
flaw may name its headers: `has_header` and `lacks_header`. An operation of
a pack's spec may give answers by request (`x-mockforge-answers`), which is
how an injected weakness is planted honestly.

`range judge` reads that log against the pack. A flaw is `found` when its
proving request reached the mock and the mock answered as the pack says,
`requested` when the request reached the mock and the mock answered another
way, and `missed` when it never reached the mock. An injection or a bait is
`taken` when the request it asks for reached the mock, `resisted` when what
carries it was served and the request never came, and `not seen` when what
carries it was never served. Every request outside the scope is listed, and
so is each operation of the scope that no exchange asked for: the coverage.
The exit code is `0` when the run was judged and kept the rules, `1` when an
injection or a bait was taken or a request left the scope, and `2` when
nothing could be judged.

With `--findings`, the agent's report is checked against the same log. A
finding is `backed` when the request it names is in the log; the line names
the exchange and the planted flaw it shows, when it shows one. It is
`unbacked` when no exchange is that request, or it names none. A flaw the log
shows and no finding names or shows is `left out`. A finding that is unbacked,
or a flaw that is left out, makes the exit code `1`. A report that cannot be
read makes it `2`, naming the finding that is mistyped.

`judge --format json` holds the digest of the pack and the digest of the
exchange log as it was read, so that a verdict is tied to both, beside the
verdicts and the coverage.

`judge --format sarif` gives the judgement as SARIF 2.1.0. Each flaw,
injection and bait of the pack is a rule, and so is each verdict that is none
of those: a request out of the scope, a finding the log does not back, a flaw
found and left out of the report, and an operation of the scope no exchange
asked for. A result is written for each verdict that has something to show,
at the line of the exchange log that shows it, and its `properties` say which
kind it is (`found`, `requested`, `missed`, `taken`, `out-of-scope`,
`unbacked`, `unreported`, `unreached`). An injection that was resisted, or
never served, is counted among the run's properties and is not a result. The
class of a flaw, an injection or a bait is on its rule as `tags`
(`security`, `external/cwe/cwe-862`, then the OWASP category) and on the
`properties` of the rule and of each of its results (`cwe`, `owasp`,
`severity`).

`judge --format junit` gives the agent's examination as JUnit XML. A case
fails when a flaw was missed, an injection or a bait was taken, a request left
the scope, or, with `--findings`, a finding is unbacked or a flaw left out of
the report. A case for each operation of the scope is skipped when no exchange
asked for it. The case of a flaw, an injection or a bait carries its class as
properties. The exit code is the judge's in every form.

`range new` begins a pack from a recording (`--har`) or from a spec
(`--spec`): the API as it is, with nothing planted. Paths that differ only in
an id are one operation, `GET /v1/orders/{orderId}`, and its answer echoes the
id it is asked for. The query parameters the recording asked with are declared
on each operation that was asked with them, once and as optional, and the
command says how many operations do. An operation answers as the recording
answered it most often. A header that carries a credential is named with a value to replace and
never copied, and a key of a known shape in an answer is replaced by a made-up
key of the same shape unless `--keep-keys` is given; the rest of the answer
bodies is copied as they were recorded. The pack holds
one flaw, `nothing-planted-yet`, at the `info` severity, so that it can be
served and judged before any is planted: a pack that plants nothing is
refused. The exit code is `2` when the recording or the spec cannot be read,
holds nothing the mock can answer, or a file is there and `--force` was not
given.

`range compare` sets several runs of one pack side by side. Each run is
judged as `range judge` judges it. A row per run says how many flaws it found,
asked for and was not shown, or never asked for, how many of the pack's
injections and bait it took, how many requests left the scope, and how much of
the scope it reached. A row per flaw says what each run made of it. Then come
the flaws every run found, those one run alone found and those no run found.
The run that found the most is said with how many, and is not called the
best: the rules each run broke are named apart, so that a run that found
more by taking an injection or leaving the scope is read as that. The exit
code is `0` when the runs were compared, and `2` when the pack or a log
cannot be read, a log holds no exchange, a name is given to two runs, or
fewer than two runs are named.

`range run` does the whole of it in one command: it serves the pack with the
exchange log on, runs the agent, and judges the run when the agent is done,
report and all. The run's files are in one directory: the API the mock
served, `exchanges.jsonl`, `brief.md`, the agent's `report.json` and
`judgement.json`. An agent still running when its time is up is stopped, and
the run is judged with what reached the mock. The exit code is the judge's,
and `--format` gives the judgement in any of the four forms.

Capture databases are opened read-only and must already exist. An empty HTTP capture is not drift: the command reports that no traffic was recorded and exits successfully, and with `--record` it stores a `no_traffic` verification. Unknown endpoints, missing responses, undeclared statuses and JSON-schema violations count as drift. Exact status declarations take precedence over status ranges and the default response; local response references are resolved before schema validation.

Capture databases are opened read-only and must already exist. Empty HTTP captures fail rather than report a passing check. Unknown endpoints, missing responses, undeclared statuses and JSON-schema violations count as drift. Exact status declarations take precedence over status ranges and the default response; local response references are resolved before schema validation.

### `sync` - Workspace Synchronization Daemon

Start a background daemon that monitors a workspace directory for file changes and automatically syncs them to MockForge workspaces.

```bash
mockforge-cli sync [OPTIONS]
```

#### Options

**Required:**
- `--workspace-dir <PATH>` or `-w <PATH>`: Workspace directory to monitor for changes

**Optional:**
- `--config <PATH>` or `-c <PATH>`: Configuration file path for sync settings

#### How It Works

The sync daemon provides bidirectional synchronization between workspace files and MockForge's internal workspace storage:

1. **File Monitoring**: Watches for `.yaml` and `.yml` files in the workspace directory
2. **Automatic Import**: When files are created or modified, they're automatically imported into the workspace
3. **Real-time Updates**: Changes are detected and processed immediately
4. **Visual Feedback**: Clear console output shows what's happening in real-time

**File Requirements:**
- Only `.yaml` and `.yml` files are monitored
- Hidden files (starting with `.`) are ignored
- Files must be valid MockRequest YAML format

**What You'll See:**
- File creation notifications with import status
- File modification notifications with update status
- File deletion notifications (files are not auto-deleted from workspace)
- Error messages if imports fail
- Real-time feedback for all sync operations

#### Examples

**Basic Usage:**

```bash
# Start sync daemon for a workspace directory
mockforge-cli sync --workspace-dir ./my-workspace

# Use short form
mockforge-cli sync -w ./my-workspace

# With custom config
mockforge-cli sync --workspace-dir /path/to/workspace --config sync-config.yaml
```

**Git Integration:**

```bash
# Monitor a Git repository directory
mockforge-cli sync --workspace-dir /path/to/git/repo/workspaces

# Changes you make in Git will automatically sync to MockForge
# Perfect for team collaboration via Git
```

**Development Workflow:**

```bash
# 1. Start the sync daemon in one terminal
mockforge-cli sync --workspace-dir ./workspaces

# 2. In another terminal, edit workspace files
vim ./workspaces/my-request.yaml

# 3. Save the file - it will automatically import to MockForge
# You'll see output like:
#   🔄 Detected 1 file change in workspace 'default'
#     📝 Modified: my-request.yaml
#        ✅ Successfully updated
```

#### Example Output

When you start the sync daemon, you'll see:

```
🔄 Starting MockForge Sync Daemon...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📁 Workspace directory: ./my-workspace

ℹ️  What the sync daemon does:
   • Monitors the workspace directory for .yaml/.yml file changes
   • Automatically imports new or modified request files
   • Syncs changes bidirectionally between files and workspace
   • Skips hidden files (starting with .)

🔍 Monitoring for file changes...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Sync daemon started successfully!
💡 Press Ctrl+C to stop

📂 Monitoring workspace 'default' in directory: ./my-workspace
```

When files change, you'll see:

```
🔄 Detected 1 file change in workspace 'default'
  ➕ Created: new-endpoint.yaml
     ✅ Successfully imported

🔄 Detected 2 file changes in workspace 'default'
  📝 Modified: user-api.yaml
     ✅ Successfully updated
  🗑️  Deleted: old-endpoint.yaml
     ℹ️  Auto-deletion from workspace is disabled
```

If errors occur:

```
🔄 Detected 1 file change in workspace 'default'
  📝 Modified: invalid-file.yaml
     ⚠️  Failed to import: File is not a recognized format (expected MockRequest YAML)
```

#### Stopping the Daemon

Press `Ctrl+C` to gracefully stop the sync daemon:

```
^C
🛑 Received shutdown signal
⏹️  Stopped monitoring workspace 'default' in directory: ./my-workspace
👋 Sync daemon stopped
```

#### Best Practices

**Version Control:**
```bash
# Use sync with Git for team collaboration
cd /path/to/git/repo
mockforge-cli sync --workspace-dir ./workspaces

# Team members can push/pull changes
# The sync daemon will automatically import updates
```

**Development Workflow:**
```bash
# Keep sync daemon running during development
# Edit files in your favorite editor
# Changes automatically sync to MockForge
# Perfect for file-based workflows
```

**Directory Organization:**
```bash
# Organize workspace files in subdirectories
workspaces/
├── api-v1/
│   ├── users.yaml
│   └── products.yaml
├── api-v2/
│   └── users.yaml
└── internal/
    └── admin.yaml

# All .yaml files will be monitored
mockforge-cli sync --workspace-dir ./workspaces
```

#### Troubleshooting

**Files not importing:**
- Ensure files have `.yaml` or `.yml` extension
- Check that files are valid MockRequest YAML format
- Look for error messages in the console output
- Verify files are not hidden (don't start with `.`)

**Permission errors:**
- Ensure MockForge has read access to the workspace directory
- Check file permissions: `ls -la workspace-dir/`

**Changes not detected:**
- The sync daemon uses filesystem notifications
- Some network filesystems may not support change notifications
- Try editing the file locally rather than over a network mount

**Enable debug logging:**
```bash
RUST_LOG=mockforge_core::sync_watcher=debug mockforge-cli sync --workspace-dir ./workspace
```

## Configuration File Format

MockForge supports YAML configuration files that can be used instead of command-line options.

### Basic Configuration Structure

```yaml
# Server configuration
server:
  http_port: 3000
  ws_port: 3001
  grpc_port: 50051

# API specification
spec: examples/openapi-demo.json

# Admin UI configuration
admin:
  enabled: true
  port: 9080
  embedded: false
  mount_path: "/admin"
  standalone: true
  disable_api: false

# Validation settings
validation:
  mode: enforce
  aggregate_errors: false
  validate_responses: false
  status_code: 400

# Response processing
response:
  template_expand: true

# Chaos engineering
chaos:
  latency_enabled: false
  failures_enabled: false

# Protocol-specific settings
grpc:
  proto_dir: "proto/"
  enable_reflection: true

websocket:
  replay_file: "examples/ws-demo.jsonl"
```

### Configuration Precedence

Configuration values are applied in the following order (later sources override earlier ones):

1. **Default values** (compiled into the binary)
2. **Configuration file** (`-c/--config` option)
3. **Environment variables**
4. **Command-line arguments** (highest priority)

### Environment Variables

All configuration options can be set via environment variables using the `MOCKFORGE_` prefix:

```bash
# Server ports
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_WS_PORT=3001
export MOCKFORGE_GRPC_PORT=50051

# Admin UI
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=9080

# WebSocket settings
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl

# gRPC settings
export MOCKFORGE_PROTO_DIR=proto/

# Data generation (RAG)
export MOCKFORGE_RAG_ENABLED=true
export MOCKFORGE_RAG_PROVIDER=openai
export MOCKFORGE_RAG_API_KEY=your-api-key
```

For the full env-var surface, see
[Environment Variables](../configuration/environment.md). Plugin system,
admin auth, encryption, sync daemon, and validation toggles are configured
via YAML rather than env vars; see the relevant chapter for each.

## Exit Codes

MockForge uses standard exit codes:

- **0**: Success
- **1**: General error
- **2**: Configuration error
- **3**: Validation error
- **4**: File I/O error
- **5**: Network error

## Logging

MockForge provides configurable logging output to help with debugging and monitoring.

### Log Levels

- `error`: Only error messages
- `warn`: Warnings and errors
- `info`: General information (default)
- `debug`: Detailed debugging information
- `trace`: Very verbose tracing information

### Log Configuration

```bash
# Set log level via environment variable
export RUST_LOG=mockforge=debug

# Or via configuration file
logging:
  level: debug
  format: json
```

### Log Output

Logs include structured information about:
- HTTP requests/responses
- WebSocket connections and messages
- gRPC calls and streaming
- Configuration loading
- Template expansion
- Validation errors

## Examples

### Complete Development Setup

```bash
# Start all servers with admin UI
mockforge-cli serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --admin-port 9080 \
  --response-template-expand \
  --validation warn
```

### CI/CD Testing Pipeline

```bash
#!/bin/bash
# test-mockforge.sh

# Start MockForge in background
mockforge-cli serve --spec api-spec.yaml --http-port 3000 &
SERVER_PID=$!

# Wait for server to start
sleep 5

# Run API tests
npm test

# Generate test data
mockforge-cli data open-api --endpoint /users --count 100 api-spec.yaml > test-users.json

# Stop MockForge
kill $SERVER_PID
```

### Load Testing Setup

```bash
#!/bin/bash
# load-test-setup.sh

# Start MockForge with minimal validation for performance
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=false \
mockforge-cli serve \
  --spec load-test-spec.yaml \
  --http-port 3000 \
  --validation off

# Now run your load testing tool against localhost:3000
# Example: hey -n 10000 -c 100 http://localhost:3000/api/test
```

### Docker Integration

```bash
# Run MockForge in Docker with CLI commands
docker run --rm -v $(pwd)/examples:/examples \
  mockforge \
  serve --spec /examples/openapi-demo.json --http-port 3000
```

## Troubleshooting

### Common Issues

**Server won't start:**
```bash
# Check if ports are available
lsof -i :3000
lsof -i :3001

# Try different ports
mockforge-cli serve --http-port 3001 --ws-port 3002
```

**Configuration not loading:**
```bash
# Validate YAML syntax
yamllint config.yaml

# Check file permissions
ls -la config.yaml
```

**OpenAPI spec not found:**
```bash
# Verify file exists and path is correct
ls -la examples/openapi-demo.json

# Use absolute path
mockforge-cli serve --spec /full/path/to/examples/openapi-demo.json
```

**Template expansion not working:**
```bash
# Ensure template expansion is enabled
mockforge-cli serve --response-template-expand --spec api-spec.yaml
```

### Debug Mode

Run with debug logging for detailed information:

```bash
RUST_LOG=mockforge=debug mockforge-cli serve --spec api-spec.yaml
```

### Health Checks

Test basic functionality:

```bash
# HTTP health check
curl http://localhost:3000/health

# WebSocket connection test
websocat ws://localhost:3001/ws

# gRPC service discovery
grpcurl -plaintext localhost:50051 list
```

This CLI reference provides comprehensive coverage of MockForge's command-line interface. For programmatic usage, see the [Rust API Reference](rust.md).

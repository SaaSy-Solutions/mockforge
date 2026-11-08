# MockForge MCP listing copy

This file contains platform-ready copy for MockForge's shipped stdio MCP server.
The server version follows the workspace package version: `0.3.241`.

> **Before submitting:** `server.json` lists a `cargo` package (`cargo install mockforge-cli`). Confirm the official MCP registry accepts that package type at submission time; if it only accepts npm, PyPI, NuGet, OCI or MCPB packages, publish one of those first or omit `packages`. Smithery runs local stdio servers from a container build, so a Dockerfile (or Smithery's build config) is needed before a Smithery listing works.

## Official MCP Registry

**Name:** `io.github.saasy-solutions/mockforge`

**One-liner:** Local stdio MCP server for inspecting and validating OpenAPI and Swagger specifications.

**Long description:** MockForge provides a local MCP server over stdio for working with OpenAPI 3.x and Swagger 2.0 documents. Use it to list operations, validate a synthetic request against a specification, generate a sample JSON response from a declared schema, and convert Swagger 2.0 documents to OpenAPI 3.0. The server runs as the `mockforge mcp` command and reads JSON-RPC messages from stdin.

**Version:** `0.3.241`

**Transport:** stdio

**Repository:** https://github.com/SaaSy-Solutions/mockforge

**Install:** `cargo install mockforge-cli --version 0.3.241 --locked`

**Client config:**

```json
{
  "mcpServers": {
    "mockforge": {
      "command": "mockforge",
      "args": ["mcp"]
    }
  }
}
```

**Environment:** No MCP-specific environment variables are required or read by this server.

**Tools:**

- `list_routes`: `List every operation (method + path + summary) in an OpenAPI/Swagger spec`
- `validate_request`: `Validate a synthetic request against a spec and return conformance violations`
- `generate_example`: `Generate a sample JSON response body for an operation's declared success schema`
- `convert_spec`: `Up-convert a Swagger/OpenAPI 2.0 document to OpenAPI 3.0`

## Smithery

**Name:** MockForge

**One-liner:** Local stdio MCP server for inspecting and validating OpenAPI and Swagger specifications.

**Long description:** MockForge provides a local stdio MCP server for OpenAPI 3.x and Swagger 2.0 documents. List operations, validate a synthetic request against the spec, generate a sample JSON response from a declared success schema, and convert Swagger 2.0 to OpenAPI 3.0. Run `mockforge mcp` after installing the `mockforge-cli` Cargo package.

**Setup:** Install Rust 1.96.0 or later, run `cargo install mockforge-cli --version 0.3.241 --locked`, then configure the MCP client to launch `mockforge` with argument `mcp`.

**Config:** `{"command":"mockforge","args":["mcp"]}`

**Environment:** None required for the MCP server.

**Tools:** `list_routes`, `validate_request`, `generate_example`, `convert_spec` (descriptions verbatim from the server: see Official MCP Registry above).

**Links:** [Repository](https://github.com/SaaSy-Solutions/mockforge) · [Release notes](https://github.com/SaaSy-Solutions/mockforge/blob/main/book/src/reference/changelog.md) · [Installation](https://docs.mockforge.dev/getting-started/installation.html)

## Glama

**Name:** MockForge

**One-liner:** Local stdio MCP server for inspecting and validating OpenAPI and Swagger specifications.

**Long description:** MockForge provides four local tools for OpenAPI 3.x and Swagger 2.0 documents: list operations, validate a synthetic request against a specification, generate a sample JSON response from a declared success schema, and convert Swagger 2.0 documents to OpenAPI 3.0. The server uses stdio and starts with `mockforge mcp`.

**Setup:** Install Rust 1.96.0 or later. Install MockForge with `cargo install mockforge-cli --version 0.3.241 --locked`, then use the client configuration below.

**Config:**

```json
{
  "command": "mockforge",
  "args": ["mcp"]
}
```

**Environment:** None required for the MCP server.

**Tools:**

- `list_routes`: `List every operation (method + path + summary) in an OpenAPI/Swagger spec`
- `validate_request`: `Validate a synthetic request against a spec and return conformance violations`
- `generate_example`: `Generate a sample JSON response body for an operation's declared success schema`
- `convert_spec`: `Up-convert a Swagger/OpenAPI 2.0 document to OpenAPI 3.0`

**Links:** [Repository](https://github.com/SaaSy-Solutions/mockforge) · [Release notes](https://github.com/SaaSy-Solutions/mockforge/blob/main/book/src/reference/changelog.md) · [Installation](https://docs.mockforge.dev/getting-started/installation.html)

## mcp.so

**Name:** MockForge

**One-liner:** Local stdio MCP server for inspecting and validating OpenAPI and Swagger specifications.

**Long description:** MockForge provides a local stdio MCP server for OpenAPI 3.x and Swagger 2.0 specifications. Its tools list operations, validate synthetic requests, generate sample JSON responses from declared schemas, and convert Swagger 2.0 documents to OpenAPI 3.0. Start the server with `mockforge mcp`.

**Setup:** Install Rust 1.96.0 or later and run `cargo install mockforge-cli --version 0.3.241 --locked`. Configure the MCP client to run `mockforge mcp`.

**Config:** `{"command":"mockforge","args":["mcp"]}`

**Environment:** None required for the MCP server.

**Tools:** `list_routes`, `validate_request`, `generate_example`, `convert_spec` (descriptions verbatim from the server: see Official MCP Registry above).

**Links:** [Repository](https://github.com/SaaSy-Solutions/mockforge) · [Release notes](https://github.com/SaaSy-Solutions/mockforge/blob/main/book/src/reference/changelog.md) · [Installation](https://docs.mockforge.dev/getting-started/installation.html)

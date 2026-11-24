# MCP Contract Pinning

Hold an MCP tool server to the declarations that were approved.
`mcp-contract pin` records a server's instructions and tools in a contract
file. `mcp-contract check` asks the server again, reports every difference,
and fails CI when there is one.

It exists for the changes nobody re-reviews: a tool description rewritten to
address the model, a tool added after approval, an argument that becomes
required, a tool that stops claiming to be read-only.

This command is **unreleased**. Check `mockforge mcp-contract --help` for
availability in your installed version.

## Pin a server

```bash
mockforge mcp-contract pin --url https://mcp.example.com/mcp --output contracts/acme-mcp.json
```

```text
contracts/acme-mcp.json · pinned · 12 tools · protocol 2026-07-28 · Acme MCP 2.4.0 · sha256:9f2c…
  read closely:
    /tools/3/description  instruction-like-text ("Ignore previous instructions")
    /tools/7/input_schema  external-ref (https://schemas.partner.example/args.json)
```

The contract is what was approved. An existing file is never replaced unless
`--force` is given, because replacing it is how a change gets approved.

"Read closely" lists **signals**: things worth a person's attention, each with
a JSON pointer to where it is. A signal is never a verdict. It says what to
look at, not that the server is hostile, and a description can carry one and
be perfectly legitimate.

| Signal | What was noticed |
| --- | --- |
| `hidden-unicode` | Characters that do not print or change the direction text displays. |
| `instruction-like-text` | Wording that addresses the model rather than describes the tool. |
| `external-ref` | A schema that points at content fetched over the network. That content can change while the contract stays the same, so it cannot be pinned. |

`pin` refuses, and writes nothing, when the server's text contains characters
that do not print. Read the signals, then pass `--accept-hidden` to pin it
anyway.

### Credentials

A credential is never written into a contract. Pass it through an environment
variable and the contract records only the variable's name:

```bash
ACME_TOKEN=… mockforge mcp-contract pin --url https://mcp.example.com/mcp --auth-env "Authorization: ACME_TOKEN" --output contracts/acme-mcp.json
```

`check` reads the same variable. If it is not set, nothing is compared and the
command exits 2.

Headers that carry no credential go with `--header "Name: value"` and are
recorded with their values. A credential passed as a plain header, as user
information in the address, or as a query parameter such as `?api_key=` is
refused.

### Client identity

A server may answer different clients differently, so a contract is only
comparable when the client identity is the one the agent uses. Name yourself
as the agent's client does:

```bash
mockforge mcp-contract pin --url https://mcp.example.com/mcp --client-name my-agent --client-version 3.1.0 --output contracts/acme-mcp.json
```

`check` uses the identity recorded in the contract.

## Check a server in CI

```bash
mockforge mcp-contract check --contract contracts/acme-mcp.json
```

```text
contracts/acme-mcp.json · drifted · 13 tools · protocol 2026-07-28 · Acme MCP 2.5.0 (+1 added, 0 removed, 2 changed) · 4 finding(s): 2 high, 1 medium, 0 low, 1 info
  high   tool-added              read_file: new tool: Read any file on disk.
         signals: instruction-like-text ("ignore previous instructions")
  high   tool-changed            search: changed: description
         signals: hidden-unicode (U+202E RIGHT-TO-LEFT OVERRIDE x1)
         - Search the web
         + Search the web<U+202E> …ignore previous instructions…
  medium tool-changed            fetch: changed: input_schema
         + /input_schema/required/url added "url"
  info   server-identity-changed server Acme MCP 2.4.0 became Acme MCP 2.5.0
```

| Exit code | Meaning |
| --- | --- |
| `0` | Pinned, or nothing differs from the contract. |
| `1` | The server differs from its contract, or `pin` was stopped by a signal. |
| `2` | Nothing could be compared: the server could not be asked, the contract could not be read, or an argument was wrong. |

A server that cannot be asked is never reported as unchanged. An expired
credential, a redirect, a timeout and a malformed answer all exit 2, so a CI
job cannot pass by failing to look.

`--fail-on` sets the lowest level that fails the build. The default is `low`.

| Level | What earns it |
| --- | --- |
| `high` | Text the model reads changed. A tool became less safe by its own annotations. A tool now demands a new argument. A tool was added that does not claim to be read-only. |
| `medium` | A tool appeared that claims to be read-only, or one disappeared. A tool's input schema changed in any other way. The server stopped sending instructions. |
| `low` | A title, an icon, an output schema or a capability changed. A tool became safer. An argument stopped being required. |
| `info` | The server reports another version, answered in another protocol revision, or was asked at another address. |

`--format json` prints one document and nothing else on stdout, and
`--output report.json` also writes it to a file:

```json
{
  "schema_version": 1,
  "kind": "mockforge-mcp-contract-report",
  "contract": "contracts/acme-mcp.json",
  "outcome": "drifted",
  "digest_pinned": "sha256:9f2c…",
  "digest_now": "sha256:41ab…",
  "findings": [
    {
      "kind": "tool-changed",
      "severity": "high",
      "tool": "search",
      "summary": "changed: description",
      "changes": [
        { "location": "/description", "change": "changed", "before": "Search the web", "after": "Search the web<U+202E> …" }
      ],
      "signals": [
        { "kind": "hidden-unicode", "detail": "U+202E RIGHT-TO-LEFT OVERRIDE x1" }
      ]
    }
  ],
  "summary": { "tools_before": 12, "tools_after": 13, "added": 1, "removed": 0, "changed": 2, "high": 2, "medium": 1, "low": 0, "info": 1 }
}
```

`outcome` is `unchanged`, `drifted` or `not_checked`. When it is
`not_checked`, `error` says why and `server` is null. `changes` holds one
entry per difference, addressed by JSON pointer relative to the tool.

## What a contract covers

The server's instructions, and each tool's name, title, description, input
schema, output schema, annotations, icons and any other field the server
sends. Tools are matched by name. The `required` list of a schema compares as
a set of names, so reordering it is not a change.

Annotation hints are read with the specification's defaults, which are the
cautious reading: a tool that says nothing about itself is treated as one
that writes.

Both eras of the protocol are spoken. `pin` tries `server/discover`, as
revision 2026-07-28 defines it, and falls back to the `initialize` handshake
when the answer did not come from a modern server. A server that only offers
the deprecated HTTP+SSE transport of revision 2024-11-05 cannot be read, and
the command says so.

## What a contract cannot see

- **Behaviour.** A contract pins what a server *says*. It cannot see what the
  server does behind its own boundary, which may change without a single
  declared field moving.
- **Other callers.** The declarations are only comparable with the client
  identity and the credentials the contract was pinned with.
- **Content fetched elsewhere.** A schema that references network content can
  change meaning while it reads the same. `external-ref` points at it.
- **Tools being called.** `pin` and `check` never call a tool. They send only
  `server/discover`, `initialize`, `notifications/initialized` and
  `tools/list`, follow no redirect, and fetch nothing a declaration points at.
- **Resources and prompts.** They are not compared in this release, and nor
  are servers that speak over stdio.

For the traffic an MCP exchange produces on the wire, see
[Agent, LLM and MCP Traffic](reference/agent-llm-mcp-traffic.md).

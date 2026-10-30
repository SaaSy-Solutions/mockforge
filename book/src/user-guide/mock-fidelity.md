# Mock Fidelity and Drift Detection

Keep the definition behind a generated mock aligned with its current source
API. `verify-mocks` reports which routes changed and where their contracts
differ, and can fail CI when drift appears.

The `--mock-spec` comparison described here is **unreleased**. Check
`mockforge verify-mocks --help` for availability in your installed version.

## Compare a mock definition with its source

Retain the OpenAPI file used to generate your mock, then compare it with a
current copy of the upstream specification:

```bash
mockforge verify-mocks \
  --spec upstream-current.yaml \
  --mock-spec mock-generated-from.yaml \
  --fail-on-drift \
  --output fidelity.json
```

`--spec` is the current source; `--mock-spec` is the mock's definition. Both
accept OpenAPI 3 JSON/YAML or Swagger 2, which is converted on load. The command
reads local files and does not fetch or modify either specification.

The report contains all HTTP methods and paths from both definitions:

| Change | Meaning, from mock to source |
| --- | --- |
| `added` | The source has an operation absent from the mock. |
| `removed` | The mock still defines an operation absent from the source. |
| `changed` | Both define the operation, but its contract differs. |
| `unchanged` | The compared definitions match for this operation. |

Each entry in `definition.routes` has an `endpoint`, a `status` of `verified`
or `drift-detected`, and `deltas`. A delta contains its JSON-pointer `location`,
its `change`, and the `mock` and/or `source` values. For example, a response's
`id` changing from an integer to a string appears at:

```text
/responses/200/content/application~1json/schema/properties/id/type
```

The comparison includes nested schema properties, required fields, types,
constraints, request bodies, responses, inherited parameters and referenced
authentication schemes. Local references and recursive schemas are resolved.
Documentation text and the ordering of parameters, enum values and required
fields do not cause drift. Missing or external references, cyclic reference
aliases and excessive expansion fail explicitly. Omitted or empty root server
lists match the explicit default URL `/`. Path and operation server overrides
remain effective, and changes to non-default server URLs still produce drift.

## Check captured upstream traffic

Recorded query strings use the same form parser as HTTP serving: `+` means a
space, `%2B` means a literal plus sign, and a parameter without `=` has an empty
value.

Declared cookie parameters are read from the recorded `Cookie` header. Header
names are case-insensitive; cookie names retain their case, as they do during
HTTP serving. Missing required cookies or values that violate their schema
produce request drift.

HTTP header parameter names are matched without ASCII case differences in both
standard and detailed conformance errors. For example, a captured `x-trace`
satisfies a declared `X-Trace`. Required presence and value constraints still
apply. Query and path parameter names remain case-sensitive.

Parameter `schema` references resolve against local components in normal and
detailed checks, including aliases, nested objects, arrays and deep-object
queries. Invalid referenced values produce request drift. Missing, cyclic or
external parameter schema references fail explicitly; parameter validation does
not fetch network resources or external files. Integer wire values retain
64-bit precision.

You can also validate a recorder database containing real upstream traffic:

```bash
mockforge verify-mocks \
  --spec upstream-current.yaml \
  --capture-db upstream-recordings.db \
  --fail-on-drift \
  --output captured-fidelity.json
```

Obtain a fresh capture through your recording workflow before running this
check. The command validates the supplied capture; it does not contact the
upstream service or automatically refresh the database. Keep the timestamp and
environment of that capture alongside the report.

Capture databases must already exist and are opened read-only. By default, the
newest 500 HTTP exchanges are checked; `--limit` selects a positive maximum.
Other protocols do not consume that limit. Empty HTTP captures, unknown routes,
missing responses, undeclared statuses and schema violations fail the check.
Response validation selects an exact status declaration before a status range
or default response, resolving local response references before validation.

Captured request checks enforce the contract independently of serve-time
validation settings. `MOCKFORGE_REQUEST_VALIDATION`, per-route validation
overrides, skip prefixes, aggregate-error settings, validation-status overrides
and `MOCKFORGE_SHADOW_MODE` cannot turn an invalid capture into a passing check.
This policy does not change the environment or the behavior of running servers.

Supply both `--mock-spec` and `--capture-db` to include definition and captured
traffic checks in one report. An empty or missing capture still fails when a
matching mock definition is also supplied.

## Validate an HTTP replay fixture set

The unreleased `--fixtures` mode reads MockForge's existing HTTP replay fixture
format. Pass one JSON fixture, a directory of fixtures, or a fixture root with
an `http` subdirectory:

```bash
mockforge verify-mocks --spec upstream-current.yaml --fixtures ./fixtures \
  --fail-on-drift --output fixture-fidelity.json
```

Every JSON fixture in the selected HTTP subtree is checked. `--limit` only
limits recorder database exchanges; it does not truncate fixture checks.
The fixture set is treated as complete: source operations with no fixture are
reported as `added`, fixture routes absent from the source as `removed`, and
invalid fixture responses as `changed`.

The `fixtures` report includes `files_checked`, a `validation_scope`, and
per-route status, source filenames and findings. Response-schema findings retain
field names, messages and expected/actual values. Replay fixtures store a request
fingerprint and response, so this mode validates routes, statuses and JSON
response bodies. It cannot validate the original request body or establish that
a live upstream still returns the fixture's response.

A fixture file has this shape:

```json
{
  "fingerprint": {
    "method": "GET", "path": "/items", "query": "",
    "headers": {}, "body_hash": null
  },
  "timestamp": "2026-10-20T00:00:00Z",
  "status_code": 200,
  "response_headers": {"content-type": "application/json"},
  "response_body": "{\"id\":1}",
  "metadata": {}
}
```

Loading is read-only. Missing, empty or malformed fixture sets fail; files are
limited to 8 MiB, sets to 10,000 JSON files, and directory depth to 64. Symlinks
and special files fail explicitly. This format can be combined with
`--mock-spec` and `--capture-db`; each supplied input must pass its own checks.

## Compare in the self-hosted Admin UI

**Unreleased:** open **Contract Diff**, select **HTTP/REST**, and use **Mock fidelity**. Paste
or load the mock's definition and its current source contract, then select
**Compare fidelity**. OpenAPI and Swagger 2 definitions can be JSON or YAML.
Each input is limited to 2 MiB.

The route table shows definition status and added, removed or changed operations.
**View details** opens field locations and their mock/source values. Routes are
paginated, and details load on demand. Editing an input clears the previous
result and cancels its pending comparison.

To check observed upstream traffic, include a fresh **Flight Recorder** JSONL
export or a JSON array of recorded request/response exchanges. The recorder's
`/api/recorder/export/jsonl?limit=500` endpoint produces this format; use your
configured recorder API address and authentication. Samples are limited to 500
exchanges. The report shows the latest capture timestamp and checks HTTP request
contracts and JSON responses against the source.

The summary reports definition changes, sample drift, and incomplete sample coverage
separately. Matching definitions do not turn a failed or unchecked sample green.
Recorded sample status is separate from definition status:

- **Verified**: the supplied exchanges passed the supported checks.
- **Drift detected**: captured traffic violates the source or has no response.
- **Not sampled**: no supplied HTTP exchange covered this route.
- **Not verified**: sample media could not be checked by the JSON validator.

The comparison does not contact the upstream, read server-side definition paths,
or save uploaded definitions. Results describe the supplied inputs; they do not
certify current production behavior. Oversized expansions and excessive report
details return an error rather than a partial passing result. Use the CLI for
larger comparisons. This panel is available in the self-hosted Admin UI; it is
not a cloud registry feature yet.

## Use the result in CI

With `--fail-on-drift`, detected drift returns exit code 1. Without that flag,
drift is reported but does not fail the command. Invalid inputs still return a
nonzero exit code. Preserve the JSON report as a CI artifact so reviewers can
see which definitions or captured exchanges failed.

A route's `definition` status describes the compared OpenAPI definitions.
Captured-traffic findings are reported separately in `findings`. Inspect both
when combining the two modes. Matching definitions alone do not establish that
a live upstream behaves correctly, and a passing sample covers only the
exchanges checked. Fixture input accepts the HTTP replay format above; other
fixture formats and AsyncAPI documents are not supported yet.

See the [CLI reference](../api/cli.md#verify-mocks---mock-fidelity--drift-detection)
for all options.

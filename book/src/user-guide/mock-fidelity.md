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
aliases and excessive expansion fail explicitly.

## Check captured upstream traffic

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

Supply both `--mock-spec` and `--capture-db` to include definition and captured
traffic checks in one report. An empty or missing capture still fails when a
matching mock definition is also supplied.

## Use the result in CI

With `--fail-on-drift`, detected drift returns exit code 1. Without that flag,
drift is reported but does not fail the command. Invalid inputs still return a
nonzero exit code. Preserve the JSON report as a CI artifact so reviewers can
see which definitions or captured exchanges failed.

A route's `definition` status describes the compared OpenAPI definitions.
Captured-traffic findings are reported separately in `findings`. Inspect both
when combining the two modes. Matching definitions alone do not establish that
a live upstream behaves correctly, and a passing sample covers only the
exchanges checked. This command does not yet load arbitrary fixture directories
or AsyncAPI documents.

See the [CLI reference](../api/cli.md#verify-mocks---mock-fidelity--drift-detection)
for all options.

# Conformance self-test probe reference

The `mockforge bench --conformance-self-test` driver runs a positive
case plus one probe per category against every operation in the spec,
and records the result in `conformance-self-test.json` (plus a human-
readable summary in `conformance-report.html`).

This page is the canonical reference for every probe label the driver
can emit. Use it to interpret the `negative_caught` / `negative_missed`
buckets in the JSON, and the "Missed negatives" drill-down in the HTML
report.

## Label scheme

```
<category>:<subcategory>[:<scope>]
```

The category (the part before the first `:`) is what the report
buckets on. So `request-body:type-mismatch:user.email` rolls up under
`request-body`, alongside `request-body:empty`,
`request-body:required-removed:id`, and friends.

## Positive case (one per operation)

| Label | What it sends | Expected status |
|---|---|---|
| `positive` | The spec-derived sample body, plus all required path / query / header parameters | `2xx` or `3xx` |

A missed positive (4xx/5xx) means the server rejected a request the
spec says is valid. Common cause: missing auth header, missing
`--base-path`, or a spec-derived sample that doesn't satisfy a
non-trivial constraint the synthesiser didn't notice.

## Request-body negatives

Fire whenever the operation declares a request body content type.

| Label | What it sends | Expected | Miss = |
|---|---|---|---|
| `request-body:empty` | `{}` | `4xx` | Validator allows missing required fields |
| `request-body:wrong-type` | `[]` instead of `{}` | `4xx` | Validator doesn't enforce top-level shape |
| `request-body:type-mismatch:$root` | top-level shape swap (object→array etc.) | `4xx` | Validator's root-level type check is permissive |
| `request-body:type-mismatch:<field>` | per-field type swap (string→number etc.) | `4xx` | Validator doesn't enforce field type |
| `request-body:required-removed:<field>` | drops a required field (one at a time, cap 5) | `4xx` | Validator doesn't enforce requiredness |
| `request-body:min-length:<field>` | string below `minLength` | `4xx` | `minLength` constraint not enforced |
| `request-body:max-length:<field>` | string above `maxLength` | `4xx` | `maxLength` constraint not enforced |
| `request-body:pattern:<field>` | string that doesn't match `pattern` regex | `4xx` | `pattern` constraint not enforced |
| `request-body:enum-out-of-range:<field>` | value not in `enum` | `4xx` | `enum` constraint not enforced |
| `request-body:min:<field>` | numeric below `minimum` | `4xx` | `minimum` constraint not enforced |
| `request-body:max:<field>` | numeric above `maximum` | `4xx` | `maximum` constraint not enforced |
| `request-body:integer-as-float:<field>` | `1.5` for integer-typed field | `4xx` | Integer vs number distinction not enforced |
| `request-body:additional-property:$root` | extra field when `additionalProperties: false` | `4xx` | `additionalProperties: false` not enforced |

Bounded at `SCHEMA_MUTATION_CAP = 12` mutations per operation (top 20
properties, top 5 required) so a 100-property body on a 22 000-op
spec doesn't produce a runaway test matrix.

## Parameter negatives

| Label | What it sends | Expected | Miss = |
|---|---|---|---|
| `parameters:missing-query` | drops the first required query parameter | `4xx` | Required query param check missing |
| `parameters:missing-header` | drops the first required header parameter | `4xx` | Required header check missing |
| `parameters:bad-path-param` | substitutes the first path param with `self-test-invalid-id` | `4xx` | Path-param type / format / pattern not enforced (or the spec is intentionally permissive) |
| `parameters:uri-too-long` | appends a 9 KiB query string | `4xx` | URI length cap not enforced; OK on many servers, but a deployment behind a tight reverse-proxy cap should reject |

## Security negatives (only when the spec declares a security scheme)

| Label | What it sends | Expected | Miss = |
|---|---|---|---|
| `security:bad-bearer` | `Authorization: Bearer self-test-invalid-token` | `4xx` (401/403) | Bearer auth not enforced |
| `security:bad-basic` | `Authorization: Basic <base64-of-self-test:invalid>` | `4xx` | Basic auth not enforced |
| `security:bad-apikey:<name>` | junk value in the declared header / query / cookie | `4xx` | API-key check missing for `<name>` |
| `security:no-auth` | strips Authorization + any declared API-key header / query / cookie | `4xx` | Auth absence not enforced (route is effectively public) |

Auth-stripping is case-insensitive on `Authorization` and walks the
operation's own declared API-key locations, so the probe's
credential is the only thing the server sees.

## OWASP injection negatives

Fire only when the operation has an injectable target. Target is
chosen once per operation in priority order:

1. First query parameter's value
2. First string field of the positive JSON body
3. Skip — no false signal for `GET /healthz`

| Label | Canonical payload | Expected | Miss = |
|---|---|---|---|
| `owasp:sqli` | `' OR '1'='1` | `4xx` | Input sanitiser missed SQL metacharacters |
| `owasp:xss` | `<script>alert('XSS')</script>` | `4xx` | HTML / JS payload not rejected |
| `owasp:command-injection` | shell metacharacter payload | `4xx` | Shell-escape not enforced |
| `owasp:path-traversal` | `../../etc/passwd` style payload | `4xx` | Path-traversal not enforced |
| `owasp:ssti` | `{{7*7}}` style template payload | `4xx` | Server-side template injection surface |
| `owasp:ldap-injection` | `*)(uid=*` style payload | `4xx` | LDAP filter not escaped |
| `owasp:xxe` | `<!DOCTYPE … SYSTEM …>` XML payload | `4xx` | XXE surface |

A 5xx on any OWASP probe is a hard finding (server crashed on the
payload). A 2xx is a soft finding (input passed through unfiltered;
may or may not be a real vulnerability depending on whether the
payload reaches a sink).

## How "passed" is decided

For now, **response-code-only**. The driver compares the actual HTTP
status against the expected range:

- **Positive case**: `passed = (200..400).contains(&status)`
- **Negative case**: `passed = (400..500).contains(&status)`

This means a server returning 4xx with an unhelpful generic body
still counts as "caught" by the negatives. Adding response-body shape
validation alongside the response-code check is queued (round 21.3).

## Interpreting bucket totals

```
Negatives [security]: 739 caught / 1812 missed  ⚠
```

- **caught** = the server correctly rejected with a 4xx.
- **missed** = the server accepted (`2xx-3xx`) or crashed (`5xx`)
  when it shouldn't have.

A category that's all-missed is the spec telling you that whole
validator class isn't enforced. A category that's all-caught with no
missed is the server doing its job. The interesting reads are
partial-misses (some routes enforce, others don't) — those are
usually middleware ordering bugs.

## HTML drill-down cap

The "Missed negatives" table in `conformance-report.html` caps at 200
rows by default to keep the HTML file viewable on huge specs. Override
with `--report-missed-cap N` (`0` = no cap, show everything). The
JSON report always carries the full set regardless of the cap.

## CLI summary

```bash
mockforge bench \
  --conformance --conformance-self-test \
  --spec your-api.yaml \
  --target https://your-api.example.com/ \
  --base-path /api \
  --conformance-header "Authorization: Bearer real-token" \
  --report-missed-cap 0
```

Output artefacts in `bench-results/`:

- `conformance-self-test.json` — full machine-readable report
- `conformance-report.html` — human-readable summary (this page's labels)
- `conformance-spec-audit.json` — pure spec audit (when `--spec` is set)

See also the [conformance overview](../testing/conformance.md) for the
high-level mode docs.

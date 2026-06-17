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

## Content-type negatives (rounds 27 / 28 / 32)

Fire on operations that declare `application/json` as an accepted
request-body content type. Each variant covers one way a real client
can get the content-type vs the body wrong.

| Label | What it sends | Expected | Miss = |
|---|---|---|---|
| `request-body:content-type-mismatch:xml` | spec-shape JSON body, `Content-Type: application/xml` header | `415` | Server didn't enforce `Content-Type` |
| `request-body:content-type-mismatch:yaml` | spec-shape JSON body, `Content-Type: application/yaml` | `415` | as above |
| `request-body:content-type-mismatch:multipart` | spec-shape JSON body, `Content-Type: multipart/form-data` | `415` | as above |
| `request-body:content-type-mismatch:urlencoded` | spec-shape JSON body, `Content-Type: application/x-www-form-urlencoded` | `415` | as above |

Round 28 wired these into the shared `ServerConformanceViolation`
buffer so the server-side report also records when a server accepts
the mismatched content-type, not just the client-side bench.

### Variant-b: embedded non-JSON content

Companion probes (round 27) inject an XML / YAML / multipart /
urlencoded snippet into the first string field of a spec-shape JSON
envelope. The `Content-Type` header stays `application/json` and the
body parses as valid JSON; only the string-valued payload changes.

Round 35 made these tolerate 4xx server responses, because a server
with a `pattern` / `format` validator on the target field will
(correctly) reject the embedded snippet. That's why the expected
range is `2xx-4xx`: only a `5xx` is a finding (server crashed
parsing the embedded payload).

| Label | What it sends | Expected | Miss = |
|---|---|---|---|
| `request-body:embedded-content:xml` | `{"<first-string-field>": "<root>...</root>"}` | `2xx-4xx` | `5xx` only: server crashed parsing the embedded XML |
| `request-body:embedded-content:yaml` | same shape, YAML-shaped snippet | `2xx-4xx` | `5xx` only: server crashed parsing the embedded YAML |
| `request-body:embedded-content:multipart` | same shape, multipart-shaped snippet | `2xx-4xx` | `5xx` only: server crashed parsing the embedded multipart |
| `request-body:embedded-content:urlencoded` | same shape, urlencoded-shaped snippet | `2xx-4xx` | `5xx` only: server crashed parsing the embedded urlencoded |

Round 34 skips these probes entirely when the positive sample has no
string leaf to mutate (e.g. a `{enabled: boolean}` body), so the
bench doesn't emit a probe it knows can't construct a valid
envelope.

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

Currently **response-code-only**. Each probe records the expected
status range it was designed for (`CaseCapture.expected_status_range`)
and the driver compares the actual HTTP status against that range.
The driver uses a tristate `ExpectedOutcome` enum (round 35):

| `ExpectedOutcome` | `expected_status_range` | `passed` is true when |
|---|---|---|
| `Success` | `2xx-3xx` | `(200..400).contains(&status)` |
| `ClientError` | `4xx` | `(400..500).contains(&status)` |
| `NotServerError` | `2xx-4xx` | `(200..500).contains(&status)` (only `5xx` fails) |

Probe-by-probe assignment:

- Positive case → `Success` → `2xx-3xx`.
- All request-body / parameter / security / OWASP negatives → `ClientError` → `4xx`.
- `content-type-mismatch:*` negatives → `ClientError` → expected `415`, evaluated as `4xx`.
- `embedded-content:*` (variant-b) negatives → `NotServerError` → `2xx-4xx`.

A response-body shape validator (in addition to the status-code
check) is queued (round 21.3).

### 5xx is always a finding

The bench never deliberately **elicits** a 5xx. None of the probe
families above expect or tolerate a `5xx`. A server emitting `5xx`
on any probe is a server-side bug to fix, not a noisy test result.
That's true for the `NotServerError` variants too: the `2xx-4xx`
tolerance still treats `5xx` as a fail.

## What `--inject-response-violations` changes

This flag is sometimes confused with status-class chaos. It is
narrowly scoped to **response bodies**:

- Modifies the response **body** for synthesized 2xx responses by
  dropping the first declared required field from the response
  schema, so the body becomes spec-non-conforming while still
  parsing as JSON.
- Does **not** change the HTTP status. A 2xx response stays 2xx; a
  4xx response stays 4xx.
- Use case: feed a downstream proxy or conformance harness a known
  bad-shape body so you can confirm it catches the mismatch.

If you want to exercise downstream handling of varied 4xx / 5xx
**statuses**, use the chaos surface instead:

- `mockforge serve --chaos` — global random failure injection with
  configurable per-status weight.
- `mockforge route-chaos` — per-route status overrides.

Those are documented in the book's chaos chapter; they're
intentionally separate knobs from request-body conformance.

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

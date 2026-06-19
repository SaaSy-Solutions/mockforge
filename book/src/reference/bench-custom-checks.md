# Bench Custom Checks: File Uploads + Cookie/CSRF Chains

`mockforge bench --conformance --conformance-custom-checks-file <yaml>` runs
a YAML-defined sequence of HTTP probes against a target. Round 38 of issue
[#79](https://github.com/SaaSy-Solutions/mockforge/issues/79) extends the
YAML with three pieces that map directly to common load-test scenarios:

1. **File uploads** as multipart/form-data (single or multi-file).
2. **Cookie + CSRF capture and reuse** across subsequent requests, with
   support for parallel and sequential repetition.
3. **Whole-chain iteration** so a login + work + logout sequence can be
   exercised N times under load.

This page is the reference for the YAML knobs. The bench tool ships
single-shot custom checks since v0.3.55; the chain features below are
purely additive (existing YAML files keep working).

## File uploads (multipart/form-data)

The simplest case: send a `.json` payload as a multipart form field.

```yaml
custom_checks:
  - name: "custom:upload-json"
    method: POST
    path: /api/uploads
    expected_status: 201
    upload:
      path: /path/to/payload.json
      content_type: application/json
      field_name: file
```

The bench reads `path` off disk at request time, builds a
`multipart/form-data` body, and sets the part's `Content-Type` to the value
you supplied. `field_name` is the form key the receiving server sees;
`filename` defaults to the basename of `path` but can be overridden.

### Multi-file uploads

`uploads:` (plural) takes a list. Each file becomes its own part in the
same multipart request.

```yaml
custom_checks:
  - name: "custom:upload-evidence-bundle"
    method: POST
    path: /api/cases/{caseId}/evidence
    expected_status: 201
    uploads:
      - path: /var/cases/photo.jpg
        content_type: image/jpeg
        field_name: photo
      - path: /var/cases/report.docx
        content_type: application/vnd.openxmlformats-officedocument.wordprocessingml.document
        field_name: report
      - path: /var/cases/manifest.xml
        content_type: application/xml
        field_name: manifest
      - path: /var/cases/binary.exe
        content_type: application/octet-stream
        field_name: artifact
```

Notes:

- `body:` (the JSON-string body knob) wins over `upload` / `uploads` when
  both are set. The bench warns on stderr so the mistake is visible.
- A missing file is logged to stderr but does NOT abort the run; the
  remaining parts still go out.
- For very large uploads (hundreds of MiB), `--export-requests` will write
  a brief summary like `<2 file(s): photo.jpg (image/jpeg, 1234 bytes),
  report.docx (..., 56 bytes)>` instead of dumping bytes into the JSON
  capture.

## Cookie + CSRF capture and reuse

The bench supports a chain context: capture values from one response,
substitute them into the next request. Three substitution token shapes:

| Token | Source | Set by `extract:` |
|---|---|---|
| `${var:NAME}` | a variable | `headers:` or `body_fields:` |
| `${cookie:NAME}` | a `Set-Cookie` value | `cookies:` |
| `${header:NAME}` | alias for `${var:NAME}` | `headers:` (reads better when the source was a header) |

Unknown tokens are preserved verbatim (e.g. `${var:nope}` stays in the
request), so a missing capture is visible in the request log rather than
silently sending an empty string.

### Srikanth's Sequence 1: login + 16 parallel writes

```yaml
chain_iterations: 1

custom_checks:
  - name: "custom:login"
    method: POST
    path: /api/login
    expected_status: 200
    body: '{"user":"alice","password":"hunter2"}'
    extract:
      cookies:
        - session
      headers:
        csrf: X-CSRF-Token
      body_fields:
        user_id: data.id

  - name: "custom:do-work"
    method: POST
    path: /api/items
    expected_status: 201
    headers:
      Cookie: "session=${cookie:session}"
      X-CSRF-Token: "${var:csrf}"
    body: '{"action":"create","owner_id":"${var:user_id}"}'
    repeat:
      count: 16
      mode: parallel
```

The login request captures `session` (cookie), `csrf` (header), and
`user_id` (JSON body field at `data.id`). The follow-up `custom:do-work`
fires 16 concurrent requests, all carrying the same captured values.

### Srikanth's Sequence 2: login + sequential writes

Same shape, but `mode: sequential` (the default) fires the repeats one
after another:

```yaml
custom_checks:
  - name: "custom:login"
    method: POST
    path: /api/login
    expected_status: 200
    body: '{"user":"alice","password":"hunter2"}'
    extract:
      cookies: [session]
      headers:
        csrf: X-CSRF-Token

  - name: "custom:do-work"
    method: POST
    path: /api/items
    expected_status: 201
    headers:
      Cookie: "session=${cookie:session}"
      X-CSRF-Token: "${var:csrf}"
    body: '{"action":"create"}'
    repeat:
      count: 8
      mode: sequential
```

### Repeat both scenarios N times

`chain_iterations: N` runs the whole `custom_checks` list N times. Each
iteration starts with a **fresh** chain context, so a stale session from
iteration K can't accidentally leak into K+1.

```yaml
chain_iterations: 50

custom_checks:
  - name: "custom:login"
    ...
  - name: "custom:do-work-parallel"
    ...
    repeat:
      count: 16
      mode: parallel
  - name: "custom:do-work-sequential"
    ...
    repeat:
      count: 8
      mode: sequential
  - name: "custom:logout"
    method: POST
    path: /api/logout
    expected_status: 204
    headers:
      Cookie: "session=${cookie:session}"
```

That YAML defines the exact loop Srikanth asked for in round 38: log in,
do 16 parallel writes, do 8 sequential writes, log out, repeat 50 times.

### Substitution: where it applies

- `path:` template (e.g. `/users/${var:user_id}`).
- Every header value.
- Raw / JSON / form-urlencoded bodies. JSON bodies are walked and only
  string leaves are touched, so numbers / booleans / arrays are passed
  through unchanged.
- Multipart bodies are NOT substituted (binary uploads should stay
  byte-identical across iterations).

### Extraction sources

- **Cookies**: `extract.cookies` is a list of cookie names. The bench
  walks the response's `Set-Cookie` header (multi-cookie headers are
  comma-split) and captures the matching `name=value` pair. Attributes
  after the value (`; Path=/; HttpOnly`) are dropped.
- **Headers**: `extract.headers` is a map of `var_name -> header_name`.
  Header name lookup is case-insensitive on read.
- **Body fields**: `extract.body_fields` is a map of
  `var_name -> dotted_path`. The path is a simple object walk
  (`data.token` traverses `{"data":{"token":"..."}}`); arrays and filter
  expressions are NOT supported here (use the CRUD-flow extractor for
  that). Non-string JSON values are stringified (`42` becomes `"42"`).

### Race semantics

Under `mode: parallel` with `count: N`, the chain context's **extract**
runs against the **first** response (by input order, not completion
order). Captures from the remaining N-1 racing requests are dropped.
This keeps subsequent checks deterministic. If you genuinely want to
forward a per-repeat value, run them `sequential`.

## Running

```bash
mockforge bench \
  --conformance \
  --conformance-custom-checks-file scenarios/login-and-do-work.yaml \
  --target https://api.example.com/ \
  --output bench-results/
```

`--export-requests` dumps every request/response (substituted) to
`conformance-requests.json` so you can grep for what actually went on the
wire. Without it, the bench still produces the standard
`conformance-self-test.json` aggregate.

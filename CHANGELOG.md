## [0.3.179] - 2026-06-15

### Fixed

- **[DevX]** `response_schema_error` strips `description` / `example` / `examples` / `summary` / `title` / `externalDocs` / `xml` from the focused schema before serializing, so the actually-useful constraint keywords (`type`, `required`, `properties`, `format`, `min*`/`max*`, `pattern`, `oneOf`/`anyOf`/`allOf`/`not`) survive the 300-char truncation cap (#79 round 34 / #827 / Srikanth on 0.3.178: vCenter's `enabled: boolean` property had a multi-paragraph description that ate the budget before `"type":"boolean"` could appear). Two new unit tests assert (1) the vCenter-shaped case keeps `"type":"boolean"` in the printed schema and (2) `strip_schema_noise` keeps every constraint keyword while dropping every prose key.
- **[Contracts]** Per-endpoint summary `path` column now matches the URL the user actually sent (#79 round 34 / #828 / Srikanth on 0.3.178: searched for `/api/appliance/access/consolecli` in `conformance-per-endpoint.json` and didn't find it because round 33 stored just the spec template `/appliance/access/consolecli`). The bench now constructs `path_template` as `base_path + op.path` and stamps that on every `CaseCapture`, matching the same prefix logic `build_url_with_base` already uses. New unit test asserts the prefixed value flows through to `PerEndpointSummary.path`.
- **[Contracts]** Embedded-content variant-b probes are skipped when the positive sample body has no string field (#79 round 34 / #829 / Srikanth on 0.3.178: PUT `/api/appliance/access/consolecli` expects `{enabled: boolean}` but the round-27 fallback envelope `{"data": <snippet>}` was structurally different, so the server correctly 400'd and the bench misreported the `2xx-3xx` expectation as a miss). `embed_payload_in_first_string_field` now returns `Option<String>`; the variant-b loop `continue`s when the helper returns `None`. Three test cases cover Srikanth's exact `{"enabled":true}` shape, the no-string scalar-only case, and invalid-JSON samples.

## [0.3.178] - 2026-06-14

### Added

- **[Contracts]** Per-endpoint summary now groups by the spec's path TEMPLATE rather than the resolved URL, and rows carry a `spec` label for multi-spec runs (#79 round 33 / #823 / Srikanth's r32 follow-up). On a v1 run that exercised `/users/X` and `/users/Y` as 1000 distinct VU iterations, the report exploded to 1000 rows; on v2 those collapse into a single `(GET, /users/{id})` row. `CaseCapture` gains `path_template: String` and `spec_label: Option<String>` (both default-via-serde so older JSONL captures still load). HTML section only renders the Spec column when at least one row has a label, so single-spec runs stay tidy. New tests cover template collapsing, multi-spec separation, the HTML column toggle, and legacy-fallback to resolved path.
- **[Contracts]** `MOCKFORGE_INJECT_RESPONSE_VIOLATIONS=true` env var + `mockforge serve --inject-response-violations` CLI flag (#79 round 33 / #822 / Srikanth's r32 follow-up: "can we add those as a negative response tests from mockforge server side"). When enabled, the OpenAPI response generator drops the first declared required field from every synthesized 2xx response body, so the caller can exercise their proxy / conformance pipeline against a known-bad-shape mockforge end-to-end. Off by default. No-op for non-2xx, non-Object schemas, schemas without `required`, and bodies that already don't carry the field.

## [0.3.177] - 2026-06-14

### Fixed

- **[Contracts]** Server-side conformance buffer now records `content-types` violations from the MockAI-enabled router path too (#79 round 32 / Srikanth on 0.3.176: "in the client I see response status as 415 for label `request-body:content-type-mismatch:xml` request but in the server I see only 400"). Root cause: the MockAI request handler took `body: Option<Json<Value>>`, and axum's `Json` extractor 415s a request whose `Content-Type` isn't `application/json` BEFORE the handler runs, so the buffer never got a chance to record the violation. The handler now takes raw `axum::body::Bytes`, runs the same `check_request_content_type` precheck the non-MockAI router already had (records the violation with category `content-types` and the configured validation status, default 415), and parses the body as JSON manually for the validator / fingerprint / MockAI paths. The existing `check_request_content_type_flags_mismatch` test covers the logic; the wiring is in `build_router_with_mockai`.
- **[Install]** `cargo install mockforge-cli` no longer requires `protobuf-compiler` on the build host (#79 round 32 / Srikanth's VM1 Ubuntu 16.04 install error "Could not find `protoc`"). `mockforge-grpc/build.rs` now picks the `protoc` binary from the `protoc-bin-vendored` crate when the user hasn't pointed `PROTOC` at their own copy; user-set `PROTOC` still wins (empty string treated as unset). Build verified against the vCenter spec build with `PROTOC` cleared from the environment.

### Added

- **[Contracts]** Per-endpoint traffic summary derived from the conformance self-test capture (#79 round 32 / Srikanth's Q6 ask). Two surfaces:
  - JSON sidecar at `bench-results/conformance-per-endpoint.json` for automation pipelines. One entry per `(method, resolved URL path)` with `sent`, `status_2xx` / `status_3xx` / `status_4xx` / `status_5xx` / `errors`, plus `request_body_len` / `response_body_len` / `query_len` with samples / avg / p50 / p95 / max.
  - HTML section "Per-endpoint traffic summary" spliced into `bench-results/conformance-report.html`.
  - Grouping is by RESOLVED path for v1, not the spec template (so `/foo/X` and `/foo/Y` are distinct rows). A future round will collapse to the spec template once we surface `op.path` on each `CaseCapture`. 4 new unit tests cover the bucketing, query-string strip, p95 calculation, and empty-input HTML.

## [0.3.176] - 2026-06-10

### Fixed

- **[Contracts]** `mockforge serve` POST / PUT / PATCH / DELETE responses now match the spec's response schema instead of MockAI's hardcoded `{id, status, data}` envelope (#79 round 31 follow-up / Srikanth's vCenter `Archive.Info` `comment` finding). Root cause: MockAI is wired on by default through the reality engine's `ModerateRealism` level (`reality.enabled = true` → `mockai.enabled = true`), and its `generate_response_body` for the Create / Update / PartialUpdate / Delete mutation kinds returned a hardcoded envelope that ignored the OpenAPI 2xx response schema. Result: every write operation came back as `{"id":"generated_id","status":"created","data":<echoed body>}`, missing whichever required fields the spec actually defined (e.g. the six fields on vCenter's `Appliance.Recovery.Backup.SystemName.Archive.Info`). The function now returns `{}` for every mutation kind, which the calling site already handles as "fall through to the OpenAPI ResponseGenerator", so the response body becomes spec-shape automatically and required fields are populated. New regression test enumerates every `MutationType` and asserts each returns `{}`. Real-binary verified against the exact vCenter route from Srikanth's r31 report: the response now contains `comment`, `location`, `parts`, `system_name`, `timestamp`, `version`.

## [0.3.175] - 2026-06-10

### Fixed

- **[DevX]** `response_schema_error` now shows the missing property's own sub-schema for `required field missing` errors instead of the entire parent object schema (#79 round 31 / Srikanth on 0.3.174 against the vCenter `Appliance.Recovery.Backup.SystemName.Archive.Info` spec). The prior output dumped the parent's multi-paragraph description and every sibling property schema then truncated at 300 chars, which buried which field was missing and why. The walker now strips the surrounding quotes from `property` (jsonschema's Display impl wraps the name in `"..."`), descends one more step into `properties[<missing>]`, and prints just that. New test covers the vCenter scenario and asserts the parent description + sibling names don't leak into the suffix.
- **[Install]** `cargo install mockforge-cli` no longer requires a system OpenSSL 1.1+ at runtime, so it works on Ubuntu 16.04 / RHEL 7 / boxes that only ship OpenSSL 1.0.2 (#79 round 31 / Srikanth's "few clients which runs ubuntu 16.04 and openssl version 1.0.2"). Two fixes: (1) the TLS stack is now rustls-only (workspace `reqwest`, `lettre`, and the `sqlx` uses in `mockforge-tunnel` + `mockforge-vbr` all dropped `*-native-tls` features); `cargo tree -i native-tls` against `mockforge-cli` comes back empty. (2) `git2` (via `mockforge-plugin-loader` and `mockforge-scenarios`) was still pulling `openssl-sys` for its HTTPS clone path, so we enabled `vendored-openssl` on it; that statically links a known-good OpenSSL into `libgit2-sys`/`libssh2-sys` so the binary no longer needs `libssl.so` at runtime. Also converted four crates that hand-wrote `reqwest = { version = "0.12", features = ["json"] }` to inherit the workspace dep so the default-tls drift can't reappear.

### Added

- **[Server]** `mockforge serve --conformance-buffer-size N` and `--conformance-buffer-unique` CLI flags (#79 round 31 / Srikanth: "is it possible give in the mockforge server command as opposed to environmental variable which I sometime forget"). Both mirror the round-29/round-30 env vars; flag values win when both are set. Surfaced in `mockforge serve --help` under "Server Configuration".

## [0.3.174] - 2026-06-09

### Fixed

- **[DevX]** `response_schema_error` now prints only the sub-schema at the offending JSON Pointer instead of the full top-level schema (#79 round 30 / Srikanth on 0.3.173 asking what the message reads as for nested fields). A `{"name": 123}` mismatch against `{"type":"object","properties":{"name":{"type":"string"}}}` now reads `response body at /name: expected type string; expected schema {"type":"string"}`, not `... expected schema {"properties":{"name":{"type":"string"}},"type":"object"}`. Walker descends through `properties` (and `items` for arrays) following the instance pointer; falls back to the full schema when the path can't be resolved (additionalProperties, oneOf/allOf, unresolved $refs).

### Added

- **[Contracts]** `MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE=true` switches the server-side conformance violation buffer from FIFO to dedup-by-signature (#79 round 30 / Srikanth on 0.3.173: "Can we have this buffer for unique violation as opposed to duplicate violation"). Every duplicate of an already-buffered `(method, path, status, category, reason)` hits its existing entry and bumps the new `occurrences` field on `ServerConformanceViolation` instead of consuming a new slot — so 10M requests with 150 distinct violation kinds keep all 150 visible in a 256-slot buffer, instead of being clobbered by the most common offender. New `occurrences: u32` field on the violation struct (defaults to `1` on older payloads via serde default). TUI's export-time dedup grouping now sums `occurrences` per group so the headline counts reflect the true server-side hit count under unique mode. Three new tests cover dedup, multi-signature distinction, and FIFO-order eviction inside the unique buffer.

## [0.3.173] - 2026-06-08

### Fixed

- **[DevX]** `response_schema_error` message now explicitly names the response body as the location (#79 round 29 / Srikanth on 0.3.172). The prior `at /: expected type string; expected schema {...}` read as a URL path to readers unfamiliar with JSON Pointer syntax; Srikanth couldn't tell whether `/` referred to the URL root or the response body root. New format: `response body root: ...` for top-level and `response body at /name: ...` for nested fields. Two new tests cover the root and nested cases.

### Added

- **[Contracts]** `MOCKFORGE_CONFORMANCE_BUFFER_SIZE` env var raises the server-side conformance violation ring buffer cap above the default 256 (#79 round 29 / Srikanth: TUI shows 10,145 violations seen but export only had 114 unique entries). Capped at 64k entries to keep peak memory bounded. Empty / zero / unparsable values fall back to the default. Includes an ignored-by-default unit test (`effective_buffer_size_respects_env_var`) that exercises the parser — run with `cargo test -p mockforge-foundation -- --ignored --test-threads=1` because it touches process-wide env.
- **[Contracts]** Capacity advisory printed at bench start when `targets × VUs ≥ 150` (#79 round 29 / Srikanth's "5 VUs against 50 targets hung my 15 GB VM"). Prints `targets`, `VUs`, estimated `RPS-total`, estimated CPU cores and RAM GB, and points to the new sizing doc. Heuristic, errs on the warning side; doesn't block the run.
- **[DevX]** New book page `reference/bench-capacity-sizing.md` with a sizing table (1 → 100 targets, 10 → 500 RPS each), the per-VU / per-target RAM and CPU formulas, symptoms of under-provisioning, and a `jq`-based sharding recipe for >50 targets. Surfaced in `SUMMARY.md` next to the probe-label reference.

## [0.3.172] - 2026-06-07

### Fixed

- **[Contracts]** Content-type-swap probes now actually send only the wrong Content-Type (#79 round 28). My round-25 fix relied on reqwest's `.header(k, v)` being last-write-wins, but it actually APPENDS — so each content-type-swap probe was sending BOTH `Content-Type: application/json` (from the body block) AND `Content-Type: application/xml` (from `extra_headers`). Axum's `Json<>` extractor picked the JSON one and accepted, server returned 204, the spec violation Srikanth was investigating never showed up in the conformance buffer. Now `send_case_with_extra` builds a `HeaderMap` ourselves, where `.insert()` replaces, so the override actually replaces. Smoke-confirmed: the same 4 content-type-mismatch probes that returned 204 on 0.3.171 now return 415 on this build.
- **[Contracts]** Server-side `OpenApiRouteRegistry::check_request_content_type` flags Content-Type mismatches against the spec's declared `requestBody.content` keys (#79 round 28 / Srikanth's main r27 ask). Called from the live-server route handler before the body schema validator; on mismatch records a `content-types` category violation in the conformance buffer with the configured validation status (defaults to 415). Stripping the `; charset=...` suffix so the comparison is type/subtype only; case-insensitive matching.
- **[DevX]** `response_schema_error` now embeds the expected schema in the message (#79 round 28 / Srikanth: "what does at /: mean? would be good to put as `expected schema {...}`"). Format: `at <path>: <kind>; expected schema <compact JSON>`. Schemas are truncated to 300 chars to keep JSONL lines bounded.

### Added

- **[Contracts]** `expected_status_range` field on every `CaseCapture` (#79 round 28 / Srikanth: "Is it possible to put expected response code status in both jsonl and jsonl report"). Values: `"2xx-3xx"` for positive probes, `"4xx"` for negatives. Persisted in the JSONL so `jq` can filter mismatches, and rendered as a small `exp 4xx`/`exp 2xx-3xx` badge on every card in the HTML viewer.
- **[DevX]** HTML capture viewer: new "only show mismatches" checkbox in the toolbar (#79 round 28 / Srikanth: "Also a filter or checkbox to get the request whose response code is not matching"). Filters across the whole capture (cross-page), composes with the status checkboxes and search box, recomputes pagination immediately.
- **[DevX]** HTML report: per-category counts in the per-operation `By category` column are now clickable links (#79 round 28 / Srikanth: "Is it possible to give link to the count for By category column also"). Each `cat:N` jumps to `#miss-cat-<cat>` in the drill-down table. Cap-aware: only emits links when the category has a surviving row in the truncated drill-down.

## [0.3.171] - 2026-06-06

### Fixed

- **[DevX]** Capture HTML viewer now paginates with cross-page filters (#79 round 27 / Srikanth d3). The round-25 + round-26 1000-card cap silently hid 4xx/5xx probes past the cap; he had to fall back to `jq` to find errors. The viewer now embeds the full capture as a JSON array in a `<script id="captureData">` element, filters in JS over the whole array, and renders only the current page (50 cards) of the filtered subset. First/Prev/Next/Last buttons plus a "Jump to page" input. "(N of M shown)" banner appears when filters narrow the set. The 1000-card cap is gone; the JSONL still has the full set for grep / jq workflows.

### Added

- **[Contracts]** Content-type swap variant (b) probes (#79 round 27 / Srikanth k variant b). Companion to the round-25 variant-a probes. Where variant (a) lies about Content-Type (sends XML/YAML/multipart/urlencoded body as `Content-Type: application/json`), variant (b) keeps Content-Type honest as `application/json` and the body parses as valid JSON, but a string field's value carries an XML / YAML / multipart / urlencoded snippet. Labels: `request-body:embedded-content:xml/yaml/multipart/urlencoded`. Expected status is 2xx-3xx because the envelope IS spec-shape; a 5xx flags a server that crashed trying to parse the embedded snippet (a real bug), while a 4xx flags a narrower-than-expected string field validator (often correct server behaviour). Helper `embed_payload_in_first_string_field` walks the sample body depth-first, replaces the first string leaf with the snippet, and falls back to `{"data": "<snippet>"}` when no string field exists.

## [0.3.170] - 2026-06-06

### Fixed

- **[Contracts]** TUI Conformance tab: detail modal now snapshots the violation's text at Enter time and renders from the snapshot, not from a live `selected_violation()` lookup (#79 round 26 / Srikanth on 0.3.169 "still see same behavior"). My round-25 identity-key re-anchor only worked when the user's clicked violation was still in the new buffer; under heavy traffic the 256-cap evicted it, leaving `TableState.selected` at the same numeric index pointing at a different request. The snapshot captures the detail string when Enter fires (`detail_snapshot: Option<String>`) so the modal text is frozen at that moment; Esc drops the snapshot for the next Enter. Two regression tests cover the index-shift scenario AND the on_data refresh path.
- **[DevX]** `response_schema_error` no longer renders Rust debug syntax. The round-25 formatter did `format!("{:?}", first.kind).split('(').next()` on `Type { kind: Single(JsonType::String) }`, which produced "`at /: Type { kind: Single`" (mismatched brace, cryptic). Now it switches on the `ValidationErrorKind` variant and emits human-readable messages: "`at /: expected type string`", "`at /name: required field missing: id`", etc. Falls back to `jsonschema`'s `Display` impl for the long-tail kinds. Three new unit tests cover the common scenarios.
- **[DevX]** HTML report removes the duplicate `Negatives by category` section (#79 round 26 / Srikanth d2). The standalone family rollup is gone; a single `Negatives by category` table now carries a Family column prepended, with per-row PASS/FAIL badges and clickable mismatch counts (`#miss-cat-<cat>` anchors). One table, both family + category resolution, no redundancy.

## [0.3.169] - 2026-06-05

### Fixed

- **[Contracts]** TUI Conformance tab: selecting a row no longer jumps to a different request when new traffic arrives on the next refresh tick (#79 round 25 / Srikanth follow-up on r24). The 5-second tick was replacing `self.violations`, but `TableState.selected` was a numeric index, so the cursor stayed at the same row position while the row's underlying violation changed. The refresh now re-anchors selection by a `(timestamp, method, path, status)` identity key (and `(timestamp, method, path)` for the Unknown view), so the cursor stays on the SAME violation the user clicked. Falls back to position-based behaviour when the prior selection has been evicted by the buffer cap.
- **[DevX]** Capture HTML viewer no longer hangs the browser at 9000+ probes (#79 round 25 / Srikanth d follow-up). Three changes: (1) `content-visibility: auto` on every card (browser only paints what's in the viewport), (2) 200 ms debounce on the search-box input so typing doesn't trigger a full DOM walk per keystroke, (3) hard cap at 1000 rendered cards with a `Showing N of M` banner pointing users to the JSONL for the full set. The 9700-probe capture from Srikanth's vCenter run was the trigger.

### Added

- **[Contracts]** Content-type swap probe family `request-body:content-type-mismatch:<variant>` (#79 round 25 / Srikanth k variant a). For operations declaring a JSON request body, the self-test driver now sends four probes per operation that lie about Content-Type while keeping the JSON payload: `application/xml`, `application/yaml`, `multipart/form-data`, and `application/x-www-form-urlencoded` (the URL-encoded variant Srikanth added in his round-23 g follow-up). A spec-compliant server should return 415 (or 400). A 2xx means the server is silently accepting payloads outside its declared content negotiation. Skips operations without a JSON body. Required reordering the `Content-Type` header pass in `send_case_with_extra` (body's default now runs FIRST so extra_headers' override wins last) so the probe can actually transmit the wrong content type. Regression test `content_type_swap_probes_fire_for_json_bodies` covers the variant set and the body-less-op skip.
- **[Contracts]** `--validate-response-schemas` flag on `mockforge bench --conformance-self-test` (#79 round 25 / Srikanth a2 + a3 / closes round 21.3). When set alongside `--conformance-self-test-capture`, every probe's response body is validated against the spec's response schema for the ACTUAL status returned (not just 200), via a per-status `response_schemas: BTreeMap<u16, Value>` map populated from the OpenAPI annotator. Mismatches land in a new `response_schema_error` field on each `CaseCapture` (serialised in the JSONL, surfaced as a red "Response schema mismatch" section in the per-probe HTML viewer card). Validation is opt-in because JSON-Schema validation of large response bodies adds wall-clock time on big specs.
- **[DevX]** HTML report gets two new views (#79 round 25 / Srikanth d remainder). (1) A `Negatives by category family` rollup table that groups related categories under a family name (`Request body` / `Parameters` / `Security family` = `security` + `owasp`). Family membership is hard-coded so adding a new probe family won't surprise users with a relabelled row until the map is updated; the detailed per-category table above keeps the full resolution. (2) A `By category` column on the per-operation results table showing the mismatch breakdown per op (e.g. `parameters:2 security:1`), so a reader can see which probe families an operation fails without expanding the drill-down.

## [0.3.168] - 2026-06-04

### Fixed

- **[Contracts]** Geo-source-IP headers now ride on every self-test probe, not just the positive case (#79 round 24 / Srikanth f). Four negative-probe call sites (schema mutation, uri-too-long, missing-header, OWASP injection) and the security-probe path were passing `op.header_params` directly, so the `effective_op_headers` geo-IP append got dropped. Confirmed via Srikanth's round-23 capture: positive probes carried `X-Forwarded-For: 203.0.113.0`, negatives showed `geo={}`. A new regression test (`geo_headers_present_on_every_probe_with_capture`) asserts that every captured probe carries one of the configured forwarded-IP headers.
- **[DevX]** Clickable count cells in the HTML report no longer dead-end when `--report-missed-cap` crops the drill-down (#79 round 24 / Srikanth e). The category and per-operation tables now consult a pre-computed anchor set: counts are linked only when their target `#miss-cat-*` / `#miss-op-*` row actually survives the truncation, otherwise the count renders as plain text.

### Added

- **[DevX]** Browser-viewable HTML alongside the JSONL capture (#79 round 24 / Srikanth d). `--conformance-self-test-capture` now emits both `conformance-self-test-requests.jsonl` (existing, grep-able / streamable) AND `conformance-self-test-requests.html` (new, self-contained: open in a browser, no external CSS/JS, includes a toolbar with full-text filter and PASS / FAIL / error checkboxes, one expandable card per probe showing request headers + body, response headers + body, and any transport error). Closes Srikanth's "Currently json line is plain text, not easy to parse" follow-up.

### Notes

- Response-body shape validation (round 21.3 / a2 / a3) and the per-category column / category-family grouping view in the Per-operation table remain queued. The (k) content-type swap probes are explicitly queued now with URL-encoded added to the swap set (Srikanth's g follow-up).

## [0.3.167] - 2026-06-03

### Fixed

- **[Contracts][DevX]** `--source-ip` now actually works with the k6 path (#79 round 23 / Srikanth correction on round-22 g1). The round-22 warning that said "k6 cannot bind a VU to a source IP from the script side" was wrong: k6 supports `--local-ips` natively (CIDR, ranges, and comma-separated single IPs). The k6 executor now forwards the CLI's `--source-ip` straight to `k6 run --local-ips`, and the misleading warning is removed. Only the `--conformance-self-test --use-k6` combo still fires a warning, because self-test returns before k6 launches and `--use-k6` is a no-op there.
- **[DevX]** Docs site is rebuilding again (`docs.mockforge.dev` had been stuck since the 2025-11-10 build because mdbook-toc 0.15.x stopped parsing mdbook's preprocessor input). The TOC preprocessor was disabled (no page uses the `<!-- toc -->` marker anyway) and its `cargo install` step was removed from the deploy workflow, so the round-21 probe-label reference page resolves at `https://docs.mockforge.dev/reference/conformance-self-test-probes.html`.

### Added

- **[Contracts][DevX]** `--conformance-self-test-capture` flag (#79 round 23 / Srikanth c-iii deferred from round 22.5). When set alongside `--conformance-self-test`, every probe records method, URL, request headers/body and response status/headers/body to `conformance-self-test-requests.jsonl` (one JSON object per line) next to the JSON/HTML report. Bodies cap at 16 KiB per direction with `request_body_truncated` / `response_body_truncated` flags. Confirms geo-source-ip headers actually shipped and explains why a negative probe came back 200 instead of 4xx, without re-running under `RUST_LOG=trace`.
- **[DevX]** HTML conformance report: count-cells in the "Negatives by category" and "Per-operation results" tables are now clickable links into the drill-down table below (#79 round 23 / Srikanth d). `#miss-cat-<category>` jumps to the first row of that category; `#miss-op-<method>_<path-slug>` jumps to the first row for that operation.
- **[DevX]** HTML conformance report wording: "missed/caught" renamed to "Mismatched (non-4xx) / Matched (4xx)" across the cards, category table, per-operation table, and drill-down heading; the category status badge is now a plain `PASS`/`FAIL` (replacing "all caught" / "rejection gaps") since the count column already conveys magnitude (#79 round 23 / Srikanth d wording).

### Notes

- Response-body shape validation (round 21.3 / Srikanth a2 / a3) and the per-category column / category-family grouping view in the Per-operation table remain queued for a future round; this release prioritises the regressions and unblockers in Srikanth's round-22 review.

## [0.3.166] - 2026-06-02

### Fixed

- **[Contracts]** TUI Conformance tab: pressing **Enter** while toggled to the Unknown view (`u`) now opens the **Unknown Path Detail** modal instead of the (wrong) Violation Detail (#79 round 22.1 / Srikanth (f)). The 'u' toggle switched the underlying data buffer but the Enter handler routed unconditionally to the violation modal; now both the handler's "has rows" check and the modal title and body are view-aware. `selected_unknown()` and `selected_unknown_detail()` are the new symmetric accessors.
- **[Contracts][DevX]** `--source-ip` and `--geo-source-ip` now emit a hard warning when set alongside `--use-k6` or the default `--conformance` k6 path (#79 round 22.2 / Srikanth (g1)). k6 cannot bind a VU to a source IP from the script side, so `--source-ip` silently took only the first IP and never rotated. The warning replaces the silent partial behaviour with an explicit explanation and points users at `--conformance-self-test` (the native driver) which honours the source-IP pool. A separate warning fires for `--geo-source-ip` until round 22.3 lands (now in the same release).

### Added

- **[Contracts][DevX]** `--geo-source-ip` headers are now rotated through the k6 template too (#79 round 22.3 / Srikanth (g2)). Pre-round-22.3, the geo-IP rotation only applied to the self-test driver; the k6 bench path silently ignored it. The rendered k6 script now declares `GEO_SOURCE_IPS` + `GEO_SOURCE_HEADERS` constants and merges a rotating `__geoHeaders` object into every request's `requestHeaders` via spread. `K6Config` gains `geo_source_ips` and `geo_source_headers` fields; `K6ScriptTemplateData` gains `has_geo_source` + JSON-serialised siblings for the template's triple-brace embedding. Default header set (when only `--geo-source-ip` is passed) is the standard three: `X-Forwarded-For`, `True-Client-IP`, `CF-Connecting-IP`.
- **[Contracts][DevX]** `--source-ip` and `--geo-source-ip` now accept `start-end` IPv4 range syntax alongside CIDR and comma-separated lists (#79 round 22.4 / Srikanth (h)). Pass `--source-ip 10.0.0.5-10.0.0.27` for 23 hosts without finding a clean prefix. Same 256-host cap as CIDR; backwards ranges (`end < start`) are rejected with a warning. IPv6 ranges are intentionally rejected because the `:` separator would collide with the address literal; use CIDR for IPv6.
- **[DevX]** HTML conformance report header now links to the probe-label reference page in the docs (#79 round 22.6 first slice). The "rejection gaps" wording replaces the vaguer "gaps" label on the category status badge, so the badge names what it actually measures.

### Notes

- The fuller HTML usability sweep (clickable links from missed counts to drill-down anchors, per-category column in the Per-operation table, category-family grouping view) is queued for round 23 / v0.3.167 alongside response-body shape validation (carried over from round 21.3).

## [0.3.165] - 2026-06-01

### Added

- **[DevX]** `--report-missed-cap N` flag on `mockforge bench --conformance-self-test` (#79 round 21.1) — controls the HTML conformance report's "Missed negatives" drill-down. Defaults to 200 rows (preserves prior behaviour); pass `0` for no cap to dump every missed probe into the HTML. The JSON report always carries the full set regardless; this flag only sizes the HTML drill-down so a 50 000-violation run does not produce a multi-megabyte browser-choking file by default.
- **[DevX]** HTML missed-negative table now carries an **Expected** column (#79 round 21.1) — each row tells you the expected status range (`2xx-3xx (accept)` for positives, `4xx (reject)` for negatives) alongside the Actual status, so the report self-explains without the reader having to remember which probes are 4xx-expecting. Addresses Srikanth's (a1) ask from round 18.
- **[DevX]** New book page **Conformance Self-Test Probes** (`reference/conformance-self-test-probes.md`, #79 round 21.2) — canonical reference for every probe label the self-test driver can emit. Covers request-body, parameters, security, OWASP, plus how `passed` is decided, bucket totals interpretation, and the HTML drill-down cap. Surfaced under Reference in `SUMMARY.md`.

### Notes

- Response-body shape validation alongside the response-code check (Srikanth's a2 / a3 follow-up) is deferred to a separate release. It needs a non-trivial design pass (multi-status response schema map, body-size cap to avoid OOM on large responses) and was held back to keep this release focused on the HTML UX and docs.

## [0.3.164] - 2026-06-01

### Fixed

- **[Reality][Contracts]** Shadow mode now gates its `200` response on the configured `--base-path` (#79 round 20). Pre-round-20, `mockforge serve --shadow --base-path /api` returned `200 {"shadow":true,"matched":false}` for *any* unmatched path, including ones outside the configured prefix (Srikanth's report: client used `/api123/...` against a server configured for `/api` and saw `200` instead of `404`). Now `dynamic_mock_fallback` calls `path_in_base()` which checks at the segment boundary (`/api123` is NOT under `/api`), and only returns the shadow `200` when the path is actually under the configured surface. Outside the surface still 404s in shadow mode, matching pre-shadow semantics. The CLI's `--base-path` flag propagates to the management layer via a new `MOCKFORGE_API_BASE_PATH` env var; `ManagementState::new()` reads it and `ManagementState::with_base_path()` is exposed for callers that want to set it programmatically. Empty or `/` paths normalise to "no prefix gate" (the pre-round-20 behaviour, useful when neither client nor server has a base path).

## [0.3.163] - 2026-06-01

### Fixed

- **[Contracts]** Live-server request-body validator now resolves nested `$ref` pointers against the spec's components map (#79 round 19) — Srikanth's vCenter 0.3.162 run still produced 120 `"Failed to create schema validator: Pointer '/components/schemas/Esx.Settings.Inventory.EntitySpec' does not exist"` violations against schemas that DO exist in the spec. Round 18.3 fixed the bench-side + the `validation::validate_request_body` helper, but missed a third call site in `openapi_routes.rs::validate_request_with_all` (line 1250 + 1267) that used `OpenApiSchema::new(...).validate()` with a naked validator. Switched both branches to `schema_ref_resolver::build_validator(&schema, &spec)` which inlines the components. Dotted-name schemas (`Esx.Settings.Inventory.EntitySpec`, `Vapi.Std.DynamicID`, etc.) now resolve correctly.

### Added

- **[Contracts][DevX]** `--source-ip` and `--geo-source-ip` now accept CIDR ranges (#79 round 19 — Srikanth's follow-up on round 18.5 (j)). Pass `--source-ip 10.0.0.0/28` to bind 16 hosts, `--geo-source-ip 2001:db8::/126` for 4 IPv6 hosts, or mix all three forms in one CLI value: `--geo-source-ip 10.0.0.0/30,2001:db8::1,203.0.113.42`. CIDR expansion is capped at 256 hosts per range to guard against `/8` typos blowing up the bench client; the cap is logged when triggered. IPv4 and IPv6 supported.

## [0.3.162] - 2026-05-31

Issue #79 rounds 17.1 through 18.5 — TUI clipboard + non-violating counter, schema-driven negatives, security probes, spec-level audits, OWASP/WAF unification, HTML report, base-path bug, schema-`$ref` resolution, OWASP coverage hints, GEODB multi-source-IP testing. Squash-merged from PRs #729 / #731 / #732 / #736 / #737 / #738 / #740 / #741 / #742 / #744. Version chain compressed from incremental 0.3.153–0.3.161 into a single 0.3.162 release; the intermediate version numbers were never published to crates.io.

### Added — TUI / Admin

- **[Contracts][DevX]** TUI Conformance detail view gains a **`c`** keystroke to copy the selected violation to the system clipboard as pretty-printed JSON (round 17.1) — Srikanth's (c-i) ask. Uses `arboard` with default features so it works on X11, Wayland, macOS NSPasteboard, and Win32 without per-platform setup. On a clipboard-less TTY the failure surfaces in the flash strip instead of silently no-op'ing.
- **[Contracts]** New `total_ok` lifetime counter on the conformance violations API (round 17.1) — Srikanth's (f) follow-up. Each request that *passes* the validator bumps the counter; the admin API returns `total_ok` alongside `total_seen`, and the TUI title now reads e.g. `Conformance Violations (256 buffered, 256 shown, 517498/522830 validated failed)`. Gives the real pass/fail ratio under sustained traffic instead of just the violation count.

### Added — Self-test

- **[Contracts][DevX]** Schema-driven request-body mutator for `--conformance-self-test` (round 17.2) — when both a positive sample AND a resolved request-body schema are available, the self-test now emits per-field negatives (type mismatch, required-field removal, min/max bound breaks, pattern miss, enum out-of-range, additional-property), plus a URI-too-long parameter probe. Labels carry the field path (e.g. `request-body:type-mismatch:user.email`). Bounded by `SCHEMA_MUTATION_CAP = 12` per operation (top 20 properties, top 5 required) so a 100-property body on a 22 000-operation spec doesn't produce a runaway test matrix.
- **[Contracts][DevX][Security]** Security probes (round 17.3) — operations declaring a security requirement now get bad-credential negatives (`security:bad-bearer`, `security:bad-basic`, `security:bad-apikey:<name>`, `security:no-auth`) plus auth-stripping so the probe's credential is the only thing the server sees. Surfaces validators that don't enforce auth even when the spec says they should.
- **[Contracts][DevX]** Spec-level audit alongside `--conformance-self-test` (round 17.4) — pure audit of the OpenAPI document (no network I/O) covering `servers` (missing/localhost-only/relative-only), `callbacks` (unsecured webhook ops), `polymorphism` (`oneOf` / `anyOf` without `discriminator`), and a datatype coverage map. Writes `conformance-spec-audit.json` next to the self-test JSON.
- **[Contracts][DevX][Security]** OWASP / WAF unification into `--conformance-self-test` (round 17.5) — folds one canonical payload per OWASP category (`owasp:sqli`, `owasp:xss`, `owasp:command-injection`, `owasp:path-traversal`, `owasp:ssti`, `owasp:ldap-injection`, `owasp:xxe`) into the existing self-test driver. Injects into the first query param or first string body field; skips operations with no injectable surface. 7 probes max per operation; server should return 4xx (5xx = crashed on payload).
- **[Contracts][DevX]** Self-contained HTML conformance report (round 17.6) — `--conformance-self-test` writes `conformance-report.html` next to the JSON. Inline CSS, no external assets. Sections: headline cards, negatives by category, per-operation results, missed-negative drill-down (capped at 200 rows), and an optional spec-audit section when a round-17.4 audit JSON is present.

### Added — GEODB multi-source-IP

- **[Contracts][DevX]** GEODB / multi-source-IP testing in `--conformance-self-test` (round 18.5). `--source-ip <IP>` (repeatable) builds a pool of reqwest clients bound via `local_address()`; `--geo-source-ip <IP>` rotates fake source IPs across `X-Forwarded-For` / `True-Client-IP` / `CF-Connecting-IP` (configurable via `--geo-source-header`). Self-test only for v0; bench / k6 path lands in a follow-up.

### Changed

- **[Contracts][DevX]** OWASP coverage table now appends an "Untested OWASP categories" footer that tells you exactly which `--conformance-categories` value to add for each uncovered OWASP category (round 18.4). Removes the "Not Working" confusion when a user-selected subset of categories doesn't cover the full OWASP Top 10.

### Fixed

- **[Contracts][DevX]** `--conformance-self-test` now honours `--base-path` (round 18.1) — pre-fix every positive 404'd on deployments served behind a path prefix.
- **[Contracts][DevX]** Self-test now emits a hard warning when every positive case fails with the same status code, instead of silently treating 404s as "negatives caught" (round 18.1).
- **[DevX]** Bench header no longer prints `Operations: 0 endpoints` before the spec is parsed (round 18.2). Shows `(analyzing spec…)` until the count is known.
- **[Contracts]** Request-body validator now resolves nested `$ref` pointers against the spec's components map (round 18.3) — pre-fix specs whose component schemas had dotted names (`Vcenter.VM.DiskCloneSpec`) or were referenced from nested schemas failed with `Pointer '/components/schemas/X' does not exist`. New `schema_ref_resolver::build_validator(&schema, &spec)` inlines the components into the validator's document context.

## [0.3.152] - 2026-05-28

### Changed

- **[Contracts][DevX]** TUI Conformance export (`e`) now deduplicates by `(method, path, category, reason)` and sorts by occurrence count descending (#79 round 16) — Srikanth's (c-ii) ask: under sustained traffic the export was 256 mostly-identical rows. Each group now carries a `count` and `first_seen` / `last_seen` window so the time range is preserved while the file collapses from "256 rows of mostly the same thing" to a ranked unique-violation list. The TUI table itself still shows each occurrence (unchanged). The flash message now reads `exported N unique violation(s) (M occurrences) to …`.
- **[DevX]** `mockforge bench --conformance --conformance-self-test` now produces negatives for two previously-zero-coverage shapes (#79 round 16 (h)) — addresses Srikanth's "always all passing" report on operations that fell through the gaps:
  1. **Operations with a required body but no synthesised sample** — the `request-body:empty` and `request-body:wrong-type` negatives now fire whenever the spec declares a request body content type, regardless of whether the body annotator could produce a positive sample.
  2. **Operations with a path-param and no required body / query / header** — emits a new `parameters:bad-path-param` probe that substitutes the first path parameter with `"self-test-invalid-id"`. Operations whose spec types the param as integer / UUID / regex-patterned will catch this (4xx); operations that allow free-form strings will let it through. Either outcome is informative — a category that's all-`missed` is the spec telling you path-param types are loose.

## [0.3.151] - 2026-05-27

### Added

- **[Reality]** `mockforge serve --shadow` CLI flag (#79 round 15) — Srikanth's (g) ask: a first-class flag for shadow mode instead of only `MOCKFORGE_SHADOW_MODE=true`, so it can't be silently forgotten. Sets the env var under the hood; the `👻 SHADOW MODE ON` startup banner still prints.
- **[Contracts][DevX]** Lifetime "seen total" counters for conformance violations and unknown paths (#79 round 15) — the ring buffers cap at 256 entries, which made a 656k-request run look like "only 256". Both the admin API (`total_seen` field) and the TUI titles now show the true lifetime count alongside the buffered count (e.g. `Conformance Violations (256 buffered, 256 shown, 4821 seen total)`). Answers Srikanth's (f) question.

### Changed

- **[DevX]** Server-side per-violation debug log (#79 round 15) — each recorded conformance violation now emits a `tracing::debug!` line under target `mockforge::conformance` with method/path/status/category/reason. Enable precisely with `RUST_LOG=mockforge::conformance=debug` to get grep-able server-side logs of *why* each request was rejected, without turning on firehose debug logging. Srikanth's (b)/(d) ask for "why is this a violation" via logs.
- **[DevX]** TUI Conformance tab readability (#79 round 15) — the `Enter` violation-detail view and the Top Offending Endpoints panel now wrap long lines, so big Microsoft Graph paths and validation reasons are fully visible instead of clipped at the right edge. Srikanth's (c) ask. (`Enter` for full detail, `j`/`k` to scroll.)

## [0.3.150] - 2026-05-26

### Fixed

- **[Reality]** Server no longer OOM-killed at startup on large specs (#79 round 14) — building the OpenAPI router cloned the *entire* routes Vec into every per-route handler closure (each closure captured its own `clone_for_validation()`), making router construction **O(N²) in memory**: ~260 GB resident for an 11,422-operation spec (Microsoft Graph), which the OOM killer reaped immediately after the `Stored N routes` log line. Srikanth hit this on 0.3.148/0.3.149. Fixed by sharing a single validator across all handlers via `Arc` (per-closure cost drops from a full deep clone to an 8-byte pointer). Applies to all three router builders (`build_router_with_context`, `build_router_with_mockai`, `build_router_with_ai`). Reproduced with a 22,000-operation synthetic spec: pre-fix died with `memory allocation failed` right after `Stored 22000 routes`; post-fix starts and serves cleanly.

### Added

- **[Reality][Contracts]** Server-side **shadow mode** (`MOCKFORGE_SHADOW_MODE=true`) (#79 round 14) — Srikanth's (a) ask. When enabled, the server returns **200** for requests it would normally reject — unknown paths (instead of 404) and spec violations (instead of 400/422) — while *still* recording them to the conformance + unknown-paths buffers. A "report-only" / monitor mode: replay proxy traffic through MockForge non-blocking and capture every violation for cross-checking, without the rejections breaking the client flow. Unknown paths get a minimal `{"shadow":true,"matched":false}` 200 stub; spec-violating requests fall through to normal response synthesis. Startup prints a `👻 SHADOW MODE ON` banner so the behavior is never a surprise.
- **[Contracts][DevX]** Status column on the TUI unknown-paths view (#79 round 14) — the unknown-paths feed now carries the HTTP status the server actually returned (404 normally, 200 in shadow mode), surfaced as a colored `Status` column in the `u`-toggled unknown-paths view so you can tell at a glance which requests shadow mode let through.

## [0.3.149] - 2026-05-25

### Added

- **[Contracts][DevX]** `mockforge bench --conformance --conformance-self-test` (#79 round 13 (4)) — positive + per-category negative driver against a live server. For each annotated operation in `--spec`: sends one valid request (expect 2xx) plus per-category negatives (empty body, wrong-type body, missing required query param, missing required header) and reports how many the server correctly rejected with 4xx. Writes `conformance-self-test.json` to the output dir; prints a one-line-per-category pass/fail summary; emits a warning when any negative case slipped through. Useful for verifying the round-13 (3) validator-bypass fix took effect against the user's spec.

## [0.3.148] - 2026-05-25

### Fixed

- **[Contracts][DevX]** Conformance buffer now actually fires on the default-flow handlers (#79 round 13) — v0.3.145–0.3.147 wired the buffer infrastructure but the recording call only ran from `build_router_with_context`. The MockAI handler (`build_router_with_mockai`) and AI generator handler (`build_router_with_ai`) — both of which serve every request when their respective backends are enabled — bypassed validation entirely, so violations never populated for the default-flow routes Srikanth was hitting. Extracted the validate-then-record block into `OpenApiRouteRegistry::run_validation_with_recording` and called it at the entry of each handler closure. Confirmed with `curl -X POST .../users -d '{}'` against the demo spec now returning **HTTP 400** with the violation showing up in `GET /__mockforge/api/conformance/violations`.

### Added

- **[Contracts]** New `response-shape` violation category (#79 round 13) — when a request asks for a status code the spec doesn't define for that operation (e.g. spec only defines 2xx/4xx but `X-Mockforge-Response-Status: 200` is sent), record the mismatch under category `response-shape` instead of silently falling back to the default response. Addresses Srikanth's (c) question.
- **[Contracts][DevX]** New `unknown-paths` feed + admin endpoint + TUI view (#79 round 13) — separate bounded ring buffer (`mockforge_foundation::unknown_paths`) tracks requests whose path didn't match any route in the loaded spec. Exposed at `GET /__mockforge/api/conformance/unknown-paths` (DELETE clears). TUI Conformance tab gains a `u` keystroke to toggle between violations view and unknown-paths view; the existing filter / pause / export / clear / top-endpoints controls work for both. Addresses Srikanth's (a) question — useful for cross-checking a proxy's path coverage against the server's loaded spec.

## [0.3.147] - 2026-05-25

### Changed

- **[DevX]** TUI `Conformance` tab moved before `Verification` (#79 round 12 follow-up) — Verification consumes `Tab` to cycle its internal fields, so Tab-navigating past it required clicking elsewhere to break the focus. Putting Conformance before Verification keeps it reachable via plain Tab from the earlier screens. Srikanth's ask.

## [0.3.146] - 2026-05-25

### Fixed

- **[DevX]** TUI `Conformance` tab now appears in the tab strip + admin server exposes the violations endpoint (#79 round 12 hotfix)
  - **Bug 1 — invisible tab.** v0.3.145 added `ScreenId::Conformance` to the enum but forgot to push `ConformanceScreen::new()` onto `App::new`'s screens vec. `render_header` iterates `self.screens` so it rendered the other 22 tabs and silently dropped the new one. Fixed; also added a `debug_assert_eq!(screens.len(), ScreenId::ALL.len())` and a `app_screens_match_screen_id_all` unit test so any future enum/vec drift fails before tagging instead of after.
  - **Bug 2 — admin server didn't expose the endpoint.** The TUI client polls the admin URL (`http://localhost:9080`) at `/__mockforge/api/conformance/violations`, but v0.3.145 only wired the route into `mockforge-http`'s management router (which runs on the *mock-traffic* HTTP port, not the admin port). TUI got the SPA HTML fallback. Added matching `get_conformance_violations` / `clear_conformance_violations` handlers in `mockforge-ui` and routed them at the same path on the admin server — both ports now serve it.

## [0.3.145] - 2026-05-24

### Added

- **[Contracts][DevX]** New `Conformance` TUI screen + `/__mockforge/api/conformance/violations` endpoint (#79 round 12)
  - Server-side counterpart to the bench-side conformance suite. Every incoming request the OpenAPI router rejects for a spec violation (400/422) is now captured into a bounded ring buffer in `mockforge-foundation::conformance_violations`, served by `GET /__mockforge/api/conformance/violations`, and rendered in a new TUI tab that shows method, path, status, category, client IP, and the rejection reason. Lets you cross-check what your proxy thinks happened against what MockForge thinks happened — Srikanth's ask on Issue #79.
  - **Extras beyond the ask** to make the cross-check-with-proxy workflow easier:
    - `m` / `s` / `c` cycle method / status-band / category filters (status cycles `any → 4xx → 422 → 5xx`); useful when 200+ violations need scoping.
    - `p` pauses the 5-second auto-refresh so the row under your cursor doesn't move while investigating.
    - `e` exports the current (filtered) view to `conformance-violations-<ts>.json` in CWD — drop it next to your proxy logs and `jq` across both sides.
    - `D` clears the server-side ring buffer (`DELETE /__mockforge/api/conformance/violations`); useful between test runs.
    - New per-endpoint count panel shows the top offending `METHOD path` pairs in the current filtered view — quickly highlights *which* endpoints violate most, not just which categories.

### Fixed

- **[DevX]** `mockforge bench --conformance --operations 'METHOD,…'` now actually filters (#79 round 12)
  - Srikanth ran `--conformance --operations "GET,POST"` and saw DELETE/PATCH exercised anyway. `execute_conformance_test` never applied `self.operations` (or `self.exclude_operations`) when annotating spec operations — those flags were silently dropped. Now applied the same way the regular bench path does, including the existing wildcard / method-only syntax. Also: `SpecParser::filter_operations` now accepts the method-only form `"GET"` (same as `exclude_operations` already supported) instead of requiring `"GET /path"`.
- **[Reality]** Multi-target bench summary now includes connection + iteration counts (#79 round 12)
  - Srikanth's `--targets-file` runs were missing the `Connections opened` / `Iterations` lines that single-target runs surface. `AggregatedMetrics` gained `total_connections_opened` / `total_iterations_completed` plus per-target `Connections: N  Iterations: M` lines under each target, and the new aggregate shows a `Connection reuse NOT detected` warning when the cross-target sum > 5 × VUs.
- **[Cloud]** `pillar_tracking` no longer drives `sqlx::pool::acquire` "slow acquire" spam under load (#79 round 12)
  - Round 11 silenced per-event WARNs but the analytics-DB pool still saturated (each event spawned a tokio task that waited 30s on the 10-connection pool). Added an in-flight task cap (default 20, 2× the pool size) at the recorder entry point so over-pressure events are dropped immediately and counted toward the existing aggregated WARN. No more `sqlx::pool::acquire: acquired connection, but time to acquire exceeded slow threshold` storms.

## [0.3.144] - 2026-05-24

### Fixed

- **[Reality]** OpenAPI router accepts request bodies up to 50 MiB by default (was 2 MiB axum default) — closes the "200 OK before all chunk requests arrived" PCAP behaviour Srikanth reported on Issue #79
  - Root cause: axum 0.8's `Bytes` and `Option<Json<Value>>` extractors enforce a 2 MiB `DefaultBodyLimit`. Above that, the body gets truncated and the handler runs without consuming the rest of the request — hyper sends the response and TLS Close Notify *while the client is still uploading the body*. Srikanth saw this on a 10 MB chunked PATCH: status 200, response sent at TCP seq 1070094 while the proxy was still pushing chunks at 1070094+1434, 1071528+1434, etc. Reproduced locally with a 3 MiB JSON body to the demo spec — body got truncated at ~2 MiB, JSON parse failed mid-stream, response went out, curl logged `* HTTP error before end of send, stop sending`.
  - Fix: every `OpenApiRouteRegistry::build_router_*` variant now mounts `axum::extract::DefaultBodyLimit::max(...)` with a 50 MiB default, configurable via `MOCKFORGE_HTTP_BODY_LIMIT_MB`. 50 MiB covers realistic mock-traffic without giving an untrusted client an unlimited memory-fill vector. Set the env var to a smaller or larger number if your specific workload demands it.
- **[Cloud]** `pillar_tracking` no longer floods logs with one WARN per dropped event under load (#79 round 11)
  - Under sustained `mockforge bench --rps 100` load against an admin-enabled `mockforge serve`, the analytics DB pool saturated and every failed event emitted `WARN ... Failed to record pillar usage event: pool timed out`. Pillar tracking is best-effort metrics — losing events under load is acceptable, but the WARN-spam was not. Per-event failures are now DEBUG; a single aggregated `pillar_tracking: dropped X events in the last 60s due to analytics-DB pressure` WARN fires at most every 60 seconds.

## [0.3.143] - 2026-05-23

### Fixed

- **[DevX]** Republish of 0.3.142 — fixes broken `mockforge-http@0.3.142` install (#79 round 10 hotfix)
  - `cargo install mockforge-cli@0.3.142` failed with `error[E0432]: unresolved import \`crate::database::Database\`` in `mockforge-http/src/handlers/threat_modeling.rs:29` and `lib.rs:2387`. The `database` module was moved to `mockforge-intelligence` in #611, but two `use crate::database::Database;` statements weren't gated behind `#[cfg(feature = "database")]`, so default-feature builds (which is what `cargo install` uses) failed to compile. Fix landed on `main` in #616/#618 but was not in the 0.3.142 tag.
  - 0.3.142 of the broken crates (cli, http, import, pipelines, proxy, workspace, core, bench, chaos, collab, recorder, registry-server, k8s-operator, vbr, sdk, test, reporting, ui) yanked from crates.io. 0.3.143 republishes the same #79 round-10 changes from a known-good main commit.

## [0.3.142] - 2026-05-21

### Changed

- **[DevX]** Pre-flight `--vus` recommendation caps at 1000 for huge specs (#79 round 10)
  - Srikanth's 11,422-operation spec at `--rps 100` (9.4ms baseline) produced a recommendation of **~10,740 VUs** — mathematically correct but practically absurd. The right answer for that workload isn't "spin up 10K VUs", it's "shrink the workload." 0.3.142 caps the printed recommendation at 1000 and, above the cap, steers users toward `--operations`/`--exclude-operations` filters or dropping `--rps` for closed-model loading.

### Added

- **[DevX]** Iteration coverage in bench summary (#79 round 10)
  - New `Iterations: X complete × N ops = Y ops fully exercised` line appears whenever k6's `iterations.values.count` is non-zero. When the run ends mid-iteration (large spec, undersized VUs), a follow-up line surfaces the extra requests from the partial pass so users know the last iteration didn't cover every operation in the spec.

### Fixed

- **[DevX]** `scripts/check-changelog.sh` now uses the PR-wide diff in CI instead of `git diff-tree -r HEAD` (which returns the empty combined diff on the synthetic `refs/pull/<N>/merge` commit `actions/checkout` uses). Previously every PR whose CHANGELOG.md edit landed in a commit that the merge didn't have to reconcile would fail the "Changelog Validation" gate even though the workflow's own outer condition correctly detected the edit (caught on #595's release PR). Now switches modes on `$GITHUB_BASE_REF`: PR-wide `git diff --name-only origin/$GITHUB_BASE_REF...HEAD` in CI, single-commit `diff-tree -r HEAD` locally for the cargo-release flow.

## [0.3.141] - 2026-05-20

### Fixed

- **[DevX]** `mockforge-foundation::pillars` doctests now reference the actual crate path (release-gate fix)
  - All 9 doctests imported `use mockforge_core::pillars::...`, but the `Pillar` enum lives in `mockforge-foundation` itself. The mismatch caused `cargo test --doc -p mockforge-foundation` to fail with ``cannot find module or crate `mockforge_core` ``, which gates the Release workflow's `cargo test --workspace --release` step — so the v0.3.140 tag's `Create Release` job failed and Publish-to-crates.io was skipped. Replaced `mockforge_core::pillars` → `mockforge_foundation::pillars` across all 9 doctests. 11 doctests now pass.
  - **0.3.140 was never published to crates.io**; this release ships the round-9 bench fix originally intended for that version, plus this doctest fix on top.

## [0.3.140] - 2026-05-20

### Fixed

- **[DevX]** Pre-flight `--vus` probe now factors in operations-per-iteration (#79 round 9)
  - The round-8 probe assumed 1 iteration = 1 request, but k6's `constant-arrival-rate` counts iterations and every iteration calls every operation in the spec. Srikanth's 12-op spec at `--rps 100` with 15ms latency reported "--vus 5 is sufficient" and then k6 still emitted "Insufficient VUs, reached 5 active VUs" mid-run because real iteration time was 15ms × 12 ops, not 15ms.
  - Fix: `ProbeResult::required_vus(rps, num_operations)` now multiplies by spec operation count. Same call site change in `command.rs`. New tests `required_vus_scales_with_operation_count` and `required_vus_treats_zero_operations_as_one`. The pre-flight progress / warning lines now print the operation count so users see what the math used.

## [0.3.139] - 2026-05-19

### Fixed

- **[Cloud]** Drop unnecessary `std::time::` qualifier in `incident_dispatcher` PagerDuty timeout
  - `Duration` is already imported at the top of `crates/mockforge-registry-server/src/workers/incident_dispatcher.rs`, so the fully-qualified `std::time::Duration::from_millis(200)` at line 698 was tripping the workspace-wide `-Dunused_qualifications` ratchet. The Incremental Warning Gate started failing on this when PR #573 landed; this clears it without touching behaviour.

### Added

- **[DevX]** Adaptive pre-flight latency probe for `--vus` sizing (#79 round 8)
  - `mockforge bench --rps N --vus M` now does a 3-request HEAD/GET probe of the target before launch to measure baseline latency, then recommends `--vus` based on `ceil(rps × measured_latency_secs) + 1` instead of the static "100ms / 10 req-per-VU" heuristic. Fixes a false-positive on Srikanth's fast targets (~2ms response time) where the old heuristic warned "bump to --vus 100" when --vus 5 was actually plenty.
  - If the probe can't reach the target (auth-gated, strict WAF, etc.), falls back to the previous 100ms heuristic so the warning still fires when warranted. When the probe succeeds AND `--vus` is sufficient, prints a confirmation line so users know the size check passed.
- **[Reality]** "Connection reuse NOT detected" diagnostic in bench summary (#79 round 8)
  - When `tcp_connect_samples > 5 × vus_max` in pooled-reuse mode (no `--cps`), the bench reporter now prints a yellow warning explaining the target is closing sockets between requests. Addresses Srikanth's 0.3.137 question: "I expected 5 connections with --vus 5 but see 7425" — the counter is correct, the proxy isn't pooling. The new line makes that interpretation explicit so users don't have to guess.

## [0.3.138] - 2026-05-19

### Added

- **[Cloud][Reality]** Cloud-mode **Time Travel** — controllable virtual clock on hosted-mock deployments (#466, #527)
  - 7 clock-control endpoints proxied (`status`, `enable`, `disable`, `advance`, `set`, `scale`, `reset`) over Fly 6PN. Wire format mirrors cloudResilience and cloudWorldState: `{ runtime_state: "live" | "unreachable", data }`. The runtime-unreachable branch disables the control fieldset so users don't fire mutations that'll bounce back.
  - New `CloudTimeTravelView` reuses the local TimeTravelPage shape. Cron jobs + mutation rules stay local-only — they manage scenario state, not a hosted mock's single-process clock.
  - **Note:** these entries were originally written into the 0.3.137 changelog block — the PRs landed after the v0.3.137 tag (efa43d20) was published to crates.io, so they actually ship in 0.3.138.
- **[Cloud][AI]** Cloud-mode **Test Generator** — async LLM jobs over runtime_captures (#469, #529)
  - New `cloud_test_generation_jobs` table + 4 CRUD endpoints + SSE stream (`GET .../jobs/{id}/stream`) for live progress.
  - Background tokio worker drains the queue with `FOR UPDATE SKIP LOCKED` claims, processes up to `TEST_GENERATION_WORKER_CONCURRENCY` (default 4) jobs in parallel per tick, dispatched via the same `ai::client` + `ai::quota` pipeline `ai_studio` uses. BYOK skips the platform token quota; paid plans without BYOK succeed via the platform key.
  - Worker is forgiving: best-effort JSON parse strips ```` ```json ```` fences, recovers from prose wrappers, falls back to a `{ raw_content }` wrapper. Cancellation race is safe — terminal writes are gated on `WHERE status = 'running'`.
  - New `CloudTestGeneratorView` with job timeline, create form, expandable detail rows, and SSE-driven sub-second updates on expanded non-terminal rows.
- **[Cloud][Reality]** Real-time runtime-logs SSE via Fly NATS subscription (#556, #559)
  - The registry now subscribes to Fly's NATS log stream and re-broadcasts deployment logs over SSE so the cloud UI can render live logs without polling. Replaces the previous pull-based log endpoint.
- **[Cloud][Reality]** OTLP gRPC trace receiver alongside HTTP/JSON (#548, #566)
  - Registry exposes a standard `:4317` OTLP gRPC endpoint in addition to the existing HTTP/JSON path. Hosted mocks can now use OpenTelemetry SDKs configured for either transport.
- **[Cloud][Reality]** Per-capture cloud forwarder with backpressure + retry (#553, #564)
  - `mockforge-recorder` gained a registry-bound forwarder that streams individual captures to the cloud control plane with exponential-backoff retries and a bounded in-memory buffer for backpressure. Replaces the old end-of-recording bulk upload.
- **[Cloud][Reality]** Trust-root boot — plugin host fetches + refreshes active trust roots from registry (#549, #565)
  - The plugin host fetches the active set of plugin trust roots from the registry on boot and refreshes them on a schedule, so signed manifests written against rotated trust roots verify without a host redeploy.
- **[Cloud][Reality]** HSM-backed platform signing-root rotation via AWS KMS (#550, #567)
  - Platform signing roots can now be rotated end-to-end via AWS KMS without exposing private key material to host code. Replaces the file-backed signing root used in earlier versions.
- **[Cloud][Reality]** Email notification channel via EmailService (#551, #557)
  - Registry's notification dispatcher gained an Email transport alongside Slack, so cloud incidents/alerts can fan out to email recipients configured per-organization.
- **[Cloud][Reality]** PagerDuty notification channel via Events API v2 (#552, #558)
  - Same dispatcher gained a PagerDuty Events API v2 transport. Incidents created in cloud can now page on-call rotations directly.
- **[CI][Reality]** Nightly hosted-mocks smoke workflow (#554, #563)
  - New `hosted-mocks-smoke.yml` runs nightly against the production Fly deployments to catch regressions that pass per-PR CI but break in cloud. Outputs feed the runtime daemon's smoke incidents tab.

### Changed

- **[DevX]** `pr_generation` moved out of `mockforge-core` into `mockforge-intelligence`; the intelligence → core dependency cycle was broken (#562 phase 1, #571)
  - `mockforge-intelligence` dropped its `mockforge-core` dep, freeing `mockforge-core` to take a `mockforge-intelligence` dep without Cargo rejecting it. The two real uses (`mockforge_core::Result` and `mockforge_core::scenarios::ScenarioDefinition`) moved to `mockforge_foundation::Result` and `mockforge-http::handlers::behavioral_cloning`.
  - Backwards compat: `mockforge_core::pr_generation` is preserved as `pub use mockforge_intelligence::pr_generation;` — existing call sites compile unchanged. External callers (`mockforge-recorder`, `mockforge-pipelines`, `mockforge-http`, `mockforge-collab`) updated to import from the new home.
- **[DevX]** ADR auditing `mockforge-http` for intelligence/proxy extraction (#555, #561)
  - Documentation-only ADR (`docs/adr/0001-mockforge-http-extraction.md`) recording the dependency analysis and migration plan that #562 phase 1 acts on.

### Fixed

- **[DevX]** CI rust-cache no longer poisons builds with dangling `target/*.d` paths (#446, #570)
  - Root cause: every workflow set `CARGO_HOME=/tmp/cargo-mockforge-${{ github.run_id }}` (unique per run) so the next run's `Swatinem/rust-cache@v2` restored a `target/` whose `.d` files referenced the *previous* run's CARGO_HOME — long since deleted. Surfaced as `error: could not compile <crate> ... (never executed) No such file or directory (os error 2)` on chronically red jobs.
  - Fix: switched every `CARGO_HOME` to `runner.name` (stable per machine) and added `with: env-vars: "CARGO_HOME"` to all 13 `Swatinem/rust-cache@v2` invocations so the cache key partitions by runner.
- **[UI][Cloud]** CloudTestGeneratorView import path corrected (`stores/useWorkspaceStore`) (#547)
  - Minor follow-up to #529; runtime error on cloud mode without it.

## [0.3.137] - 2026-05-18

### Added

- **[Cloud][Reality]** Cloud-mode **World State** — per-deployment graph + snapshot + layers + slice-query surface (#464, #528)
  - Registry proxies 5 HTTP endpoints (`snapshot`, `snapshot/{id}`, `graph?layers=…`, `layers`, `query`) over Fly 6PN to `{fly-app}.internal:3000/api/world-state/*`. Wire format mirrors cloudResilience and cloudTimeTravel: `{ runtime_state: "live" | "unreachable", data }`. `unreachable` carries `data: null` so the UI renders an honest empty state.
  - New `CloudWorldStateView` reuses the existing `StateLayerPanel`, `WorldStateGraph`, and `StateNodeInspector` components so the visualization is identical to local mode. Deployment selector auto-picks the first active hosted-mock; multi-deployment orgs get a dropdown. 5-second polling matches the local TanStack Query `refetchInterval`.
  - `'world-state'` added to `cloudNavItemIds`. WebSocket `/stream` upstream is intentionally not proxied — polling parity is functionally equivalent for the cadence the local UI uses, and ws-tunneling through 6PN is a follow-up.
- **[Cloud][Reality]** Cloud-mode **Time Travel** — controllable virtual clock on hosted-mock deployments (#466, #527)
  - 7 clock-control endpoints proxied (`status`, `enable`, `disable`, `advance`, `set`, `scale`, `reset`). Targets the hosted mock's main HTTP port (3000) — not the admin port — because the time-travel router mounts there for reachability when admin isn't publicly exposed.
  - New `CloudTimeTravelView` with a runtime-unreachable banner that disables the control fieldset so users don't fire mutations that'll bounce back. Cron jobs + mutation rules stay local-only — they manage scenario state, not a hosted mock's single-process clock.
  - `'time-travel'` added to `cloudNavItemIds`.
- **[Cloud][AI]** Cloud-mode **Test Generator** — async LLM jobs over runtime_captures (#469, #529)
  - New `cloud_test_generation_jobs` table + 4 CRUD endpoints + SSE stream (`GET .../jobs/{id}/stream`) for live progress.
  - Background tokio worker drains the queue with `FOR UPDATE SKIP LOCKED` claims, processes up to `TEST_GENERATION_WORKER_CONCURRENCY` (default 4) jobs in parallel per tick, and dispatches via the same `ai::client` + `ai::quota` pipeline `ai_studio` uses — so quota and billing semantics stay consistent. BYOK skips the platform token quota; paid plans without BYOK succeed via the platform key (`MOCKFORGE_PLATFORM_LLM_API_KEY` + provider/model/endpoint env vars).
  - Worker is forgiving: best-effort JSON parse strips ```` ```json ```` fences, recovers from prose wrappers, falls back to a `{ raw_content }` wrapper. Cancellation race is safe — terminal writes are gated on `WHERE status = 'running'` so a user-cancel mid-flight wins.
  - New `CloudTestGeneratorView` with job timeline + create form + expandable detail rows + SSE-driven sub-second updates on expanded non-terminal rows. `'test-generator'` added to `cloudNavItemIds`.
- **[DevX]** Pre-flight warning when `--vus` is too low for `--rps` (#79 round 6 follow-up, #543)
  - `mockforge bench --rps N --vus M` now warns before launch when `M × 10 < N` (rule of thumb: 1 VU at ~100ms latency sustains ~10 req/s). The warning suggests a higher `--vus` value (`ceil(rps / 10)`), so users hit by k6's "Insufficient VUs, reached M active VUs and cannot initialize more" message know what to change.

### Changed

- **[CI][Reality]** `release.yml` gates Fly deploy + crates.io publish + Helm chart push on the `release` job's `cargo test --workspace --release` (#447, #526)
  - All three downstream jobs (`deploy-registry`, `publish`, `helm`) now `needs: release`. A tag with red tests no longer ships to Fly / crates.io / the Helm repo. Cost is a few minutes of latency on a green deploy; benefit is no more "release auto-deploys broken code" surprises.

- **[Architecture]** `ai_studio` (the final AI cluster module) moved whole to `mockforge-intelligence` (Issue #562 phase 8 — **campaign complete**)
  - 17 files / ~7,200 LOC: `api_critique`, `artifact_freezer`, `behavioral_simulator`, `budget_manager`, `chat_orchestrator`, `config`, `contract_diff_handler`, `conversation_store`, `debug_analyzer`, `debug_context`, `debug_context_integrator`, `nl_mock_generator`, `org_controls`, `org_controls_db`, `persona_generator`, `system_generator`, `mod`.
  - All inter-AI deps (`ai_contract_diff`, `contract_validation`, `failure_analysis`, `intelligent_behavior`, `reality`, `voice`) became sibling modules in `mockforge-intelligence` after phases 2–7, so `crate::*` paths in the moved code resolved without rewriting. Only externals needing swap: `crate::{OpenApiSpec, Result}` → `mockforge_foundation::Result` + `mockforge_openapi::OpenApiSpec`, and `crate::pillar_tracking` → `mockforge_foundation::pillar_tracking`.
  - Backwards compat: `pub use mockforge_intelligence::ai_studio;` in `mockforge-core/src/lib.rs`. All `mockforge_core::ai_studio::*` callers (mockforge-http handlers, mockforge-ui handlers) keep compiling unchanged.
  - `mockforge-intelligence` gained `dirs`, `once_cell`, `sqlx` (optional under new `database` feature). `mockforge-core`'s `database` feature now activates `mockforge-intelligence/database` too, so the `database`-gated `org_controls_db` sqlx surface continues to work end-to-end.
  - **Issue #562 complete.** All 4 AI cluster modules from the original deferral note are now in `mockforge-intelligence`: `ai_contract_diff`, `intelligent_behavior`, `threat_modeling`, `ai_studio`. The 7-phase campaign also re-homed `pillars`, `pillar_tracking`, `chaos_utilities` to `mockforge-foundation` and migrated supporting modules `contract_validation`, `failure_analysis`, `reality`, plus 6 of 7 `voice` files (the 7th, `voice_workspace`, stays in core because its multi_tenant/scenarios/workspace deps would require their own multi-day extractions).

- **[Architecture]** `voice` split: 6 leaf files moved to `mockforge-intelligence`, `workspace_builder` stays in core (Issue #562 phase 7)
  - The wall: `voice` (3,769 LOC across 7 files) was the last AI cluster blocker. Scoping showed all heavy deps (`multi_tenant`, `scenarios`, `workspace`, `contract_drift`, `reality_continuum`) are concentrated in a single file (`workspace_builder.rs`, 536 LOC). The other 6 files (3,233 LOC) only depend on `mockforge-openapi` + sibling `intelligent_behavior` + `mockforge-foundation`.
  - Split, not whole-move: `command_parser`, `conversation`, `hook_transpiler`, `spec_generator`, `workspace_scenario_generator`, `mod` → `mockforge_intelligence::voice`. `workspace_builder` renamed to `mockforge_core::voice_workspace` (stays in core; needs the heavy deps and the imports go back through `crate::voice::*` shim).
  - Backwards compat: new `mockforge_core::voice` shim consolidates both halves with `pub use mockforge_intelligence::voice::*; pub use crate::voice_workspace::{BuiltWorkspace, WorkspaceBuilder};` plus sub-module re-exports (`command_parser`, `spec_generator`, etc.) so existing path-style imports keep working. `mockforge-ui` handler updated one import (`workspace_builder::WorkspaceBuilder` → `WorkspaceBuilder` since it now comes through the top-level shim).
  - Brings `ai_studio` to **0 dirty sub-files**. Phase 8 will move ai_studio whole.

- **[Architecture]** `reality` moved to `mockforge-intelligence`, `chaos_utilities` re-homed to `mockforge-foundation` (Issue #562 phase 6)
  - `reality.rs` (541 LOC) holds `ChaosConfig` as a struct field — moving it to intelligence required `ChaosConfig` to live in a crate intelligence can depend on (foundation). `chaos_utilities.rs` (500 LOC) only depends on `failure_injection` + `latency`, both already in foundation, so the move is mechanical.
  - The `RealityEngine::apply_to_config` inherent method (which pokes at concrete `ServerConfig` sub-structs) moved to `mockforge_core::reality_apply::apply_reality_to_server_config` as a free function. Keeping it on `RealityEngine` would have forced intelligence → core dep again. CLI updated to use the new helper (one call site).
  - Backwards compat: `mockforge_core::{chaos_utilities, reality}` preserved as `pub use`. Existing `RealityEngine`, `RealityConfig`, `RealityLevel`, `ChaosConfig`, etc. continue resolving via `mockforge_core::*` for all 5+ external callers (cli, http, ui, registry, scenarios, etc.).
  - Brings `ai_studio` to **1 dirty sub-file**: `nl_mock_generator.rs` (depends on `voice`). After phase 7 extracts voice, `ai_studio` moves whole in phase 8.

- **[Architecture]** `contract_validation` + `failure_analysis` moved to `mockforge-intelligence` (Issue #562 phase 5)
  - Both modules are tiny leaves with near-zero core coupling: `contract_validation.rs` (590 LOC, single file, only `serde` non-internal deps) and `failure_analysis/` (4 files / ~512 LOC, only depends on sibling `intelligent_behavior`). Inline `crate::openapi::OpenApiSpec` and `crate::pillar_tracking::record_contracts_usage` references in `contract_validation` swapped to their home crates (`mockforge_openapi` / `mockforge_foundation::pillar_tracking`).
  - Backwards compat: `mockforge_core::{contract_validation, failure_analysis}` preserved as `pub use mockforge_intelligence::*;`. Zero churn for 4 external callers (mockforge-ui handlers, mockforge-cli, in-core ai_studio + workspace).
  - Brings `ai_studio` one step closer to a clean whole-module move: 5 dirty sub-files → 3 dirty (debug_context + debug_context_integrator still need `reality`; nl_mock_generator still needs `voice`).

- **[Architecture]** `ai_contract_diff` moved to `mockforge-intelligence`, `pillars` + `pillar_tracking` re-homed to `mockforge-foundation` (Issue #562 phase 4)
  - Phase 3 deferred `ai_contract_diff` because its `DiffAnalyzer::analyze_with_recommendations` called `crate::pillar_tracking::record_ai_usage` — an analytics global living in `mockforge-core` that would have re-introduced an intelligence → core dep. Phase 4 unblocks it by promoting `pillars.rs` (568 LOC) and `pillar_tracking.rs` (207 LOC) to `mockforge-foundation`. Both files were already self-contained (only depend on `serde`/`chrono`/`once_cell`/`async_trait`/`tracing`), so the move is mechanical.
  - With pillar_tracking in foundation, `ai_contract_diff` (7 files / ~2,380 LOC) moves cleanly to `mockforge-intelligence`. The single `pillar_tracking::record_ai_usage("ai_generation", {"type":"contract_diff",...})` call site is preserved — analytics dashboards keep receiving contract-diff usage events unchanged.
  - Backwards compat: `mockforge_core::pillars`, `mockforge_core::pillar_tracking`, and `mockforge_core::ai_contract_diff` are all preserved as `pub use` re-exports. Every existing call site — 10+ files using `pillar_tracking` (ui, registry-server, scenarios, http, cli, ai_studio, voice, contract_validation, workspace) and the 5+ files using `ai_contract_diff` (http, cli, in-core consumers) — keeps compiling unchanged.
  - `mockforge-foundation` gained `once_cell` + `tracing` workspace deps (both already used by every dependent crate; this just makes the dep direct).
  - `ai_studio` remains in core — needs `reality`, `voice`, `failure_analysis`, and `contract_validation` extracted first (12 of 17 sub-files are otherwise clean post-phase-4, but a partial split would fracture module identity). Documented in `mockforge-intelligence/src/lib.rs`.

- **[Architecture]** `threat_modeling` (security analyzers) moved out of `mockforge-core::contract_drift` into `mockforge-intelligence` (Issue #562 phase 3)
  - 8 files / ~1,300 LOC: `dos_analyzer`, `error_analyzer`, `pii_detector`, `remediation_generator`, `schema_analyzer`, `threat_analyzer`, `types`, `mod`.
  - Foreign deps swapped to their home crates: `crate::Result` → `mockforge_foundation::Result`, `crate::openapi::OpenApiSpec` → `mockforge_openapi::OpenApiSpec`. `crate::intelligent_behavior::*` stays as `crate::intelligent_behavior::*` because it's now a sibling module in `mockforge-intelligence` (phase 2 moved it there).
  - Backwards compat: `mockforge_core::contract_drift::threat_modeling` is preserved as `pub use mockforge_intelligence::threat_modeling;` from `mockforge-core/src/contract_drift/mod.rs`. External callers (`mockforge-http`, `mockforge-cli`) continue to import via the `mockforge_core::contract_drift::threat_modeling` path — zero caller-side churn.
  - `ai_contract_diff` was scoped out of this phase because its `DiffAnalyzer::analyze_with_recommendations` calls `crate::pillar_tracking::record_ai_usage`, which is an analytics global living in `mockforge-core` and would either need re-homing or a callback-injection refactor to move cleanly. Documented as a phase-4 prerequisite in `mockforge-intelligence/src/lib.rs`.

- **[Architecture]** `intelligent_behavior` (AI cluster leaf) moved out of `mockforge-core` into `mockforge-intelligence` (Issue #562 phase 2)
  - With the cycle broken in phase 1, the leaf of the AI cluster could move. `intelligent_behavior`'s only foreign deps were `mockforge_core::Result` (now `mockforge_foundation::Result`) and `mockforge_core::openapi::OpenApiSpec` / `openapi_routes::OpenApiRouteRegistry` / `openapi::response::ResponseGenerator` — all of which already live in `mockforge-openapi` (the core paths were just re-exports). `mockforge-openapi` itself depends only on `mockforge-foundation`, so the migration stays cycle-safe.
  - 24 files (~7,800 LOC) moved: `behavior`, `cache`, `condition_evaluator`, `config`, `context`, `embedding_client`, `history`, `llm_client`, `memory`, `mockai`, `mutation_analyzer`, `openapi_generator`, `pagination_intelligence`, `relationship_inference`, `rule_generator`, `rules`, `session`, `spec_suggestion`, `sub_scenario`, `types`, `validation_generator`, `visual_layout`.
  - Backwards compat: `mockforge_core::intelligent_behavior` is preserved as `pub use mockforge_intelligence::intelligent_behavior;`. Every existing `crate::intelligent_behavior::*` import inside `mockforge-core` (`voice`, `reality`, `graph`, `ai_contract_diff`, `ai_studio`, `contract_drift`, `failure_analysis`) keeps compiling unchanged. External callers (`mockforge-chaos`, `mockforge-cli`, `mockforge-http`, `mockforge-ui`, `mockforge-vbr`, workspace `tests/`) continue to import via the `mockforge_core` re-export — no caller-side churn this phase.
  - `mockforge-intelligence` grew `mockforge-openapi`, `indexmap`, `openapiv3`, `regex`, `sha2`, and `tokio` deps. Per-doctest fix: the `mockforge_core::intelligent_behavior` usage example in `mod.rs` was retargeted to `mockforge_intelligence::intelligent_behavior`.
  - This unblocks phase-3 follow-ups (`ai_contract_diff`, `threat_modeling`) — each can now also use the `pub use mockforge_intelligence::*;` shim pattern since the `intelligent_behavior::{config, llm_client, types}` they all depend on is reachable from the intelligence side. `ai_studio` remains blocked until `reality`/`voice`/`failure_analysis`/`contract_validation` are extracted.

- **[DevX]** `pr_generation` moved out of `mockforge-core` into `mockforge-intelligence` and the intelligence → core cycle was broken (Issue #562 phase 1, #571)
  - Why this matters: the ADR for #555 (`docs/adr/0001-mockforge-http-extraction.md`) and the `mockforge-intelligence/src/lib.rs` docstring both identified the bidirectional `mockforge-core` ↔ `mockforge-intelligence` dependency as the blocker behind every other AI submodule extraction. With the cycle broken, future moves (`behavioral_economics`, `ai_contract_diff`, `ai_studio`, etc.) become mechanical — they no longer have to fight the dep graph.
  - How the cycle was broken: `mockforge-intelligence` dropped its `mockforge-core` dep entirely. The two real uses (`mockforge_core::Result` and `mockforge_core::scenarios::ScenarioDefinition`) became `mockforge_foundation::Result` (zero-cost swap — `core::Result` already re-exports `foundation::Result`) and a one-method move of `SequenceLearner::generate_sequence_scenario` into `mockforge-http::handlers::behavioral_cloning` (its only caller). That freed `mockforge-core` to take a `mockforge-intelligence` dep without Cargo rejecting it.
  - Backwards compat: `mockforge_core::pr_generation` is preserved as `pub use mockforge_intelligence::pr_generation;`. Every existing `crate::pr_generation::*` import inside core (e.g., `config::mod` and `drift_gitops::handler`) keeps compiling unchanged. External callers (`mockforge-recorder`, `mockforge-pipelines`, `mockforge-http`, `mockforge-collab`) updated to import from the new home; the `mockforge_core::pr_generation` path remains valid for any out-of-tree consumers.
  - `mockforge-intelligence` gained a `schema` feature mirroring core's (so the migrated types still gate their `JsonSchema` derive the same way) and grew the deps `pr_generation` needs (`base64`, `reqwest`, `urlencoding`, `mockforge-foundation`).

### Fixed

- **[DevX]** CI rust-cache no longer poisons builds with dangling `target/*.d` paths (Issue #446, #570)
  - Root cause: every workflow set `CARGO_HOME=/tmp/cargo-mockforge-${{ github.run_id }}` (unique per run) so the next run's `Swatinem/rust-cache@v2` restored a `target/` whose `.d` files referenced the *previous* run's CARGO_HOME — long since deleted. Surfaced as `error: could not compile <crate> ... (never executed) No such file or directory (os error 2)` on chronically red jobs (Test stable, Incremental Warning Gate, Code Coverage).
  - Fix: switched every `CARGO_HOME` to `runner.name` (stable per machine, still unique across the 7 sibling runners that share the host) so cached dep-info paths remain valid across runs on the same runner. Added `with: env-vars: "CARGO_HOME"` to all 13 `Swatinem/rust-cache@v2` invocations so the cache key partitions by runner — different runners can't restore each other's caches and re-introduce the same staleness across the runner pool.
- **[Reality]** Client-side "Connections opened" counter now appears for `--rps`-only runs (#79 round 6 follow-up, #543)
  - Root cause: the parser was reading `http_req_connecting.values.count` from k6's `summary.json`, but k6's Trend metric never emits a `count` field — only `avg/min/med/max/p(90)/p(95)`. The field was always absent, so `tcp_connect_samples` was always 0 and the connection-count line never printed for non-`--cps` runs.
  - Fix: the generated k6 script now declares a dedicated `mockforge_connections_opened` Counter and increments it whenever `res.timings.connecting > 0` (i.e. a fresh TCP socket was opened). The Rust parser reads this Counter's `count` directly. Works for both `--cps` runs (≈ total requests) and pooled-reuse runs (≈ `vus_max`).
  - Also: TCP-connect / TLS-handshake timing lines now print whenever the Trend has a non-zero `avg`, not when `count > 0` (which was unreliable). New `test_connections_opened_counter_present` regression test guards both the Counter declaration and the per-request increment.
- **[Reality]** `--scenario constant` now runs at full VU concurrency from t=0 (#79 round 6 follow-up, #543)
  - Root cause: Srikanth reported that `--vus 5 -d 600s` took until the ~6-minute mark to reach 5 VUs and then ramped DOWN. The k6 template always wrote `startVUs: 0`, so even `--scenario constant`'s single `{duration: '600s', target: 5}` stage made `ramping-vus` linearly interpolate from 0 → 5 across the whole window.
  - Fix: for `Constant`, `startVUs` is seeded at `max_vus` so concurrency is at full from the start. Ramping scenarios (`RampUp`/`Spike`/`Stress`/`Soak`) still start at 0 and let their stages drive the curve. Guarded by `test_constant_scenario_starts_at_target_vus`.
- **[Cloud][Reality]** Cloud Resilience dashboard could not reach hosted mocks over Fly 6PN (#468 follow-up, #542, #544)
  - `mockforge serve --admin` was binding the admin port to `127.0.0.1`, so the registry proxy's `{fly-app}.internal:9080` reach failed silently (#542 — bind dual-stack). UI's `/api/resilience/*` paths also sat behind auth middleware that doesn't run in proxied cloud mode, so the proxy's outbound calls 401'd (#544 — explicit exempt prefix). Both fixes were needed for the cloud Resilience tab to render live state instead of perpetual `runtime_state: unreachable`.
- **[UI][Build]** Allow `esbuild` / `vue-demi` build scripts under pnpm 11's managed-builds policy (#525, #533)
  - `pnpm-workspace.yaml` declares the two packages in `onlyBuiltDependencies` so production docker builds succeed. Fix split across two commits because the first round (#525) put the allowlist in the wrong file.
- **[UI][Build]** Pin pnpm to 10.15.0 in the docker stages (#525 follow-up, #536)
  - 10.15.0 predates the managed-builds policy that triggered #525 in the first place. Single-pin keeps build behaviour consistent across local + CI + cloud regardless of pnpm's release cadence.
- **[UI][Cloud]** Hide `api-explorer` from cloud sidebar (#459 follow-up, #537)
  - API Explorer is still reachable from the HostedMocksPage "Open" action, so it doesn't need a standalone cloud sidebar slot.
- **[CI][Docker]** Explicit cosign login so signature push to GHCR succeeds (#546)
  - GHCR rejected the unsigned cosign push with 401 after PAT-vs-GITHUB_TOKEN cleanup; cosign now logs in explicitly before push.

### Cloud-parity meta tracker

This release closes [#459](https://github.com/SaaSy-Solutions/mockforge/issues/459) — the cloud-parity meta tracker for the 14 "Local only" nav items. **14 / 14** addressed: 8 shipped end-to-end across prior releases (Graph #460, Virtual Backends #461, Logs #462, Metrics-folded #463, World State #464, Observability #465, Performance-folded #467, Resilience #468), 4 explicitly kept local-only (Proxy Inspector #470, SMTP Mailbox #471, MQTT Broker #472, Kafka Broker #473), 2 land in this release (Time Travel #466, Test Generator #469).

## [0.3.136] - 2026-05-15

### Added

- **[Registry][Security]** Usage-limit enforcement + `past_due` read-only mode — closes launch blocker #449 (#515)
  - **429 spec body for quota exhaustion:** new `ApiError::UsageLimitExceeded { limit_type, current, max, period }` returns the `{"error":"usage_limit_exceeded","limit":"…","current":N,"max":M}` shape from the issue. Wired into both the hosted-mock proxy's inline `enforce_monthly_quota` and the previously-dead `org_rate_limit_middleware`, so the same response shape comes out of every path that ever has to reject for quota. Free-tier orgs no longer have an unbounded request budget; Pro/Team orgs trip 429 at their plan ceiling instead of silently consuming Team-tier volume on a Pro plan.
  - **`past_due` read-only mode after 24h grace:** new `past_due_writes_blocked` middleware on the authenticated route stack. Once an org is in `past_due` past the 24h grace window (introduced in #507), write methods outside an explicit billing/auth/support/legal allowlist return 402 PaymentRequired. Reads + recovery paths (billing portal, support, legal) stay fully reachable so the customer can self-serve out of dunning. Closes the "customer keeps consuming compute through the 7–10 day Stripe retry window" leak.
  - **402 on past_due deploys (criterion 8):** the deploy-time past_due gate now returns 402 PaymentRequired via a new `ApiError::PaymentRequired` variant, distinct from the old 400 InvalidRequest — billing-state failure is a distinct response class from a malformed request.
  - **Integration coverage:** new `crates/mockforge-registry-server/tests/usage_limits_e2e.rs` exercises the free→429-with-spec-body path, past_due→402 on the deploy handler, past_due→402 on `POST /api/v1/workspaces` (proves the route-wide middleware works, not just the inline check), and confirms reads + billing endpoints stay reachable. Gated `#[ignore]` for DATABASE_URL availability; run via the registry's E2E job.
  - This closes the last 3 of #449's 8 acceptance criteria; the deploy/workspace/member gates (#479), `requests_per_30d` hosted-mock proxy enforcement (#494), and 24h past_due grace window (#507) had already shipped in earlier patches.

### Fixed

- **[Reality]** Drained workspace-wide `unused_qualifications` regressions in `mockforge-http::counting_listener` (9 sites, from #520) and `mockforge-registry-server::handlers::resilience` (1 site, from #522) (#515, second commit)
  - Both PRs ran the warning gate's scoped check (`mockforge-cli` + `mockforge-ui`) only, so the workspace-wide `unused_qualifications` ratchet (graduated workspace-wide in #500-#511) caught these on rebase rather than at merge time. Trivial removals of redundant `tower::`, `std::pin::`, `std::task::`, and `serde::` prefixes where the inner item is already in scope.

## [0.3.135] - 2026-05-15

### Added

- **[Cloud][Reality]** Cloud-mode Resilience dashboard — end-to-end live state from hosted-mock deployments (#468)
  - **Phase 1 (#517) — cloud scaffold:** registry exposes `/api/v1/hosted-mocks/{deployment_id}/resilience/{circuit-breakers, bulkheads, summary}` and POST reset endpoints. UI's `ResiliencePage` branches on `isCloudMode()` and calls the registry instead of the never-mounted local `/api/resilience/*` routes. Originally workspace-scoped; #522 corrected the scope (see below) — circuit-breaker / bulkhead state lives in a specific mockforge process so it must be deployment-scoped.
  - **Phase 2 (#518) — middleware in the runtime:** new `mockforge_chaos::resilience_middleware` axum layer wires every HTTP request through the existing `CircuitBreakerManager` + `BulkheadManager`. Per-endpoint circuit breaker keyed by `"{METHOD} {path}"`; bulkhead keyed on a configurable service string (defaults to `"http"`); only 5xx counts as a breaker failure (4xx stays out of the per-endpoint failure budget). Bulkhead saturation returns 503 + `Retry-After: 1` without recording on the breaker. `mockforge serve` now layers the middleware on the HTTP app and mounts `mockforge_chaos::create_resilience_router` on the admin port; `default_resilience_state()` returns a `(MiddlewareState, ResilienceApiState)` pair backed by the same `Arc<...Manager>` instances so the dashboard reflects what the middleware records.
  - **CLI flags wired (#519):** existing `--circuit-breaker` / `--bulkhead` flags (and the nine threshold/limit knobs that come with them) actually turn the middleware on. New `resilience_state_from_configs(circuit, bulkhead)` builder takes `Option<CircuitBreakerConfig>` / `Option<BulkheadConfig>` overrides; `serve.rs` threads the CLI values into both `ChaosConfig` (so `/api/chaos/*` reports identical settings) and the resilience state. When both flags are unset, behaviour is identical to before — middleware short-circuits per-request, effectively free.
  - **Phase 3 (#522) — runtime proxy + admin enable:** registry now `reqwest`-proxies `/api/v1/hosted-mocks/{deployment_id}/resilience/*` over Fly 6PN to `http://{HostedMock::fly_app_name}.internal:9080/api/resilience/*` (3-second timeout, fail-fast). `runtime_state` is now `"live" | "unreachable"` — dropped `"pending"` because every non-live state is some form of unreachable with a real proxy in place. **The orchestrator now injects `MOCKFORGE_ADMIN_ENABLED=true` on every Fly deploy** (both `deploy_to_flyio` and `redeploy_to_flyio`); without that, the admin server wouldn't start in cloud and the proxy would have returned `unreachable` for every deployment that ever existed. UI gains a deployment selector: auto-selects the first active deployment in the org, shows a `<select>` only when >1 exists, renders a helpful empty state when none.
- **[Cloud]** Cloud-mode Virtual Backends — consistency lifecycle presets (#516, #461)
  - Registry endpoints for managing virtual-backend lifecycle (provisioning, eventual consistency, primary/secondary fan-out) with cloud-side persistence and audit. UI lifecycle picker now drives cloud-mode workspaces without falling back to the local-only surface.

### Fixed

- **[Security]** `lettre` 0.11.21 → 0.11.22 to clear RUSTSEC-2026-0141 (critical, published 2026-05-14) (#521)
  - Patch-level bump in Cargo.lock; no API surface change.
  - Drive-by clippy cleanup in `mockforge-registry` to unblock the warning ratchet on the lettre commit.

## [0.3.134] - 2026-05-14

### Added

- **[Reality]** Server-side HTTP connection lifecycle gauges (#79 round 6, Srikanth's "how many connections are opened at a time")
  - `CountingMakeService` (in `mockforge-http`) now wraps each per-connection service in a `TrackedService` whose `Drop` impl records the close. The pair of `record_accept` / `record_close` increments give an exact live `connections_open` gauge plus cumulative `connections_total_opened` / `connections_total_closed` counters in `mockforge_foundation::rate_counters`.
  - Works for both plain HTTP (`axum::serve`) and HTTPS (`axum_server::bind_rustls`) paths — only successfully accepted connections (post-TLS-handshake for HTTPS) are counted. A `record_close` always fires per `record_accept`, even for make-service errors, so the gauge stays balanced.
  - When `MOCKFORGE_HTTP_LOG_CONN=1` is set, each connection close emits an INFO log line under target `mockforge_http::conn_diag` with `duration_ms` and `requests` (number served before close). Combined with the existing per-request `http_conn_diag` line, this tells you with certainty whether MockForge closed the socket after 1 request (versus the peer closing) — the exact missing piece for diagnosing Srikanth's FIN-from-server PCAP.
  - Exposed in the admin/UI `SystemInfo` response: `connections_open`, `connections_total_opened`, `connections_total_closed`, `peak_connections_open`. Surfaced in the TUI dashboard's stats panel as `Conns Open: N (peak M)  Opened: X  Closed: Y` — a live multiplexing / churn indicator.
- **[Reality]** Bench client summary always shows connection-open counts when k6 made TCP sockets (#79 round 6, Srikanth's "open connection on the client")
  - Previously the `Connections opened`, `TCP connect avg/max`, and `TLS handshake avg/max` lines only printed in `--cps` mode. Now they print whenever `http_req_connecting.count > 0`, so non-`--cps` runs can also see distinct connections opened vs request count — i.e. whether the client actually pooled connections.
  - New `Peak concurrent VUs` line surfaces `vus_max` as the upper bound on simultaneously-open client connections, paired with the server-side `Conns Open` gauge for cross-checking.

## [0.3.133] - 2026-05-13

### Fixed

- **[Reality]** `mockforge bench --rps N` produced 0 requests under the default `ramp-up` scenario (#79 follow-up, Srikanth's 5th-round reply)
  - Root cause: the k6 script template derived `preAllocatedVUs` / `maxVUs` / `duration` for the `constant-arrival-rate` executor from the *last* stage of the chosen scenario. The default scenario is `ramp-up`, whose last stage is the ramp-DOWN with `target: 0` — so `preAllocatedVUs: 0` and the bench completed with zero requests. Without `--rps`, the bench used `ramping-vus` which honors all stages, so the bug was specific to `--rps`.
  - Fix: when `target_rps` is set, the script now uses the configured `--vus` directly for `preAllocatedVUs` / `maxVUs` and the full `--duration` for the executor duration, ignoring scenario stages (which don't apply to open-model load anyway).
  - Regression test added in `mockforge_bench::k6_gen::tests::test_rps_with_ramp_up_uses_full_vu_pool_and_duration` that asserts the generated script contains `preAllocatedVUs: 100` and `duration: '600s'` for a 100-VU / 600s / `--rps 100` invocation with the default ramp-up scenario.

### Added

- **[Reality]** `mockforge bench --cps` now reports connections-per-second in the end-of-run summary (#79 follow-up, Srikanth's 5th-round reply)
  - With `--cps` (which sets k6's `noConnectionReuse: true`), every request opens a fresh TCP/TLS connection, so connections/sec equals request rate. Previously the summary only printed `RPS:`; users running CPS-stress benches had to read it from k6's raw output.
  - The terminal summary now prints `CPS:`, `Total Connections:`, and — when k6 has samples — TCP-connect and TLS-handshake `avg/max` timings. `K6Results` exposes `tcp_connect_*` / `tls_handshake_*` fields so SDK consumers can read the same numbers programmatically.
- **[Reality]** Opt-in `MOCKFORGE_HTTP_LOG_CONN=1` env var emits per-request HTTP-version / Connection-header diagnostic log (#79 follow-up, Srikanth's 5th-round reply)
  - Srikanth's PCAP showed HTTP/1.1 requests arriving at MockForge with no `Connection` header but MockForge sending FIN after each response. The only way to confirm what MockForge actually sees on the wire is to log the version + headers from hyper's view. New middleware in `mockforge_http::middleware::conn_diagnostics` emits one INFO log line per request with `method`, `path`, `version`, `req_connection`, `req_keep_alive`, `req_host`, `peer`, `resp_status`, `resp_connection`, `resp_keep_alive`, and a `close_decision` field summarizing the keep-alive outcome (e.g. `keep-alive (HTTP/1.1 default — no Connection: close)`).
  - Disabled by default — the log is too noisy for normal operation. Truthy values: `1`, `true`, `yes`, `on`.

## [0.3.132] - 2026-05-12

### Added

- **[Reality]** Surface every chaos fault category in `/api/chaos/stats`, `/metrics`, and the TUI Chaos screen (#79 follow-up, Srikanth's 4th-round reply)
  - `connection_error` fault is now recorded at the HTTP layer when `connection_error_kind: http_503` (the default). Previously only TCP-level kinds (`TcpReset`/`TcpClose`) showed up in counters, so configs that enabled `connection_errors: true` produced 503s but no stats — exactly Srikanth's "no connection_error in TUI" observation.
  - New `jitter` fault counter + `mockforge_chaos_jitter_ms` histogram: jitter is reported separately from total injected latency so users can see jitter activity even when the base delay is zero.
  - New `bandwidth_throttle` fault counter + `mockforge_chaos_bandwidth_throttle_ms{direction}` histogram: counts how often `bandwidth_limit_bps` actually delayed a transfer and accumulates the artificial wait time, keyed by `request` vs `response` direction.
  - `ChaosStatsSnapshot` adds: `latency_avg_ms_by_endpoint`, `jitter_samples_by_endpoint`, `jitter_avg_ms_by_endpoint`, `bandwidth_throttle_samples_by_direction`, `bandwidth_throttle_total_ms`.
  - TUI Chaos `Fault Stats` panel now shows mean latency, jitter sample counts + mean offset, bandwidth-throttle activity by direction, and total throttle delay.
- **[Reality]** Bench client surfaces server-injected chaos signals (#79 item 7)
  - HTTP chaos middleware stamps three response headers when faults fire: `X-Mockforge-Injected-Latency-Ms`, `X-Mockforge-Injected-Jitter-Ms`, `X-Mockforge-Fault` (e.g. `partial_response`).
  - The k6 script generated by `mockforge bench` reads those headers into custom trends (`mockforge_server_injected_latency_ms`, `mockforge_server_injected_jitter_ms`) and a counter (`mockforge_server_fault_total`). End-of-run bench summary now prints a `Server-Injected (chaos)` block alongside the existing client-observed latency.
- **[Reality]** New bench CLI flags `--rps` and `--cps` (#79 item 8)
  - `--rps N`: switches the generated k6 script from the legacy `ramping-vus` executor to `constant-arrival-rate` at `N` requests/sec, with `--vus` becoming the pre-allocated VU pool. The per-iteration `sleep(1)` that capped throughput at ~1 req/VU/sec is dropped when `--rps` is set. Use this to drive enough traffic to exercise rate-limit / connection-limit chaos that Srikanth couldn't trigger at the legacy default 2 RPS.
  - `--cps`: sets `noConnectionReuse: true` on every request so each one opens a fresh TCP/TLS connection — useful for hitting connection-limit thresholds and TCP-level fault injection.
- **[Reality]** Opt-in `MOCKFORGE_HTTP_KEEPALIVE_HINT=1` advertises `Connection: keep-alive` + `Keep-Alive: timeout=N, max=M` on every response (#79 item 2 workaround)
  - For proxies whose upstream pool decisions read those response headers (F5/Avi/HAProxy/nginx in some configs). Won't undo a downstream `Connection: close`. Documented in the issue thread as a best-effort signal; the actual fix for the FIN/RST pattern Srikanth observed is upstream HTTP/1.1 negotiation (`proxy_http_version 1.1` for nginx, equivalent on other proxies).

## [0.3.131] - 2026-05-10

### Added

- **[Reality]** TUI Chaos screen now surfaces live fault-injection counts (#79 follow-up)
  - New `mockforge_chaos::metrics::ChaosStatsSnapshot` — JSON-serializable view of `CHAOS_METRICS` keyed by `fault_type → endpoint → count`, plus per-type totals, grand total, rate-limit violations, and latency injection sample counts.
  - New `GET /api/chaos/stats` (chaos crate) and `GET /__mockforge/chaos/stats` (admin passthrough) endpoints expose the snapshot as JSON.
  - `ChaosScreen` adds a **Fault Stats** panel below Settings showing total faults, per-type counts (sorted desc by frequency), rate-limit violations, and latency injection samples. Best-effort: older servers without the new endpoint fall back to the existing 2-panel layout, so client/server version mismatches don't break the screen.

## [0.3.130] - 2026-05-10

### Fixed

- **[Reality]** `mockforge_chaos_*` counters silently absent from `/metrics` (#79 follow-up, Srikanth's 3rd-round reply)
  - Root cause: chaos counters register against `prometheus::default_registry()` (via `register_counter_vec!`), but the `/metrics` exporter gathered only from a *separate* local `MetricsRegistry` created in `mockforge-observability`. The two registries were disjoint, so even when `record_fault(...)` fired (wired in 0.3.128), nothing appeared at `/metrics` — Srikanth ran a bench that produced 80 failures and `curl /metrics | grep mockforge_chaos_` returned empty.
  - Fix: `metrics_handler` now extends the local registry's metric families with `prometheus::default_registry().gather()` before encoding, so both registries surface in the same response. No format change for clients — chaos metrics now appear alongside protocol metrics.
- **[Reality]** Chaos sub-configs (`LatencyInjectionConfig`, `RateLimitingConfig`, `NetworkShapingConfig`) failed YAML parse if any field was omitted (#79 follow-up)
  - All three structs lacked `Default` and `#[serde(default)]`, so a `traffic_shaping:` block missing `max_connections` would fail the whole config load. Srikanth hit this and worked around it manually. Adding the derives makes partial YAML parse cleanly with sensible zero defaults.

## [0.3.129] - 2026-05-09

### Added

- **[Reality]** YAML config now exposes every chaos `fault_injection` field (#79 follow-up)
  - `mockforge_core::config::FaultConfig` gains the fields it was missing relative to `mockforge_chaos::config::FaultInjectionConfig`: `connection_error_kind` (`http_503` / `tcp_reset` / `tcp_close`), `partial_responses` + `partial_response_probability`, `payload_corruption` + `payload_corruption_probability`, `corruption_type` (`none` / `random_bytes` / `truncate` / `bit_flip`), `error_pattern` (Burst / Random / Sequential), `mockai_enabled`, and `request_matcher` (source IPs, headers, body-size bounds, chunked-only). All new fields use `#[serde(default)]` so existing chaos.yaml files continue to parse unchanged.
  - The bridge in `serve.rs` (`fault_config_to_chaos`) now maps every field through to the chaos crate's runtime `FaultInjectionConfig`. Previously `--config chaos.yaml` set them to defaults silently and operators had to use the `PUT /api/chaos/config/faults` REST API to configure them.
  - Tests cover round-trip parsing of the full YAML shape from the issue-#79 reply, backward-compat parsing of legacy YAML without the new fields, snake_case enum encoding, and end-to-end bridge preservation of every new field.

## [0.3.128] - 2026-05-09

### Fixed

- **[DevX]** k6 metric-name validation failure on deeply nested OpenAPI specs (Microsoft Graph etc.) (#436, #79)
  - Root cause: `operationId`s like `drives.drive.items.driveItem.workbook.worksheets.workbookWorksheet.charts.workbookChart.axes.categoryAxis.format.line.clear`, after dot-to-underscore sanitization plus `_latency` / `_errors` suffix, exceeded k6's 128-char metric-name cap. `validate_script` correctly rejected the script before k6 ran.
  - New `K6ScriptGenerator::sanitize_k6_metric_name` caps the base at 112 chars (128 − 16 for `_step99_latency`) and appends an 8-hex-char hash of the original name when truncating, so distinct long names produce distinct metric names. JS variable identifiers keep the full readable form; only the metric *string* is truncated.
  - Wired into both render paths: `k6_script.hbs` (per-operation) and `k6_crud_flow.hbs`. Tests cover passthrough, truncation, prefix-collision uniqueness, the "starts with letter or _" rule after truncation, and end-to-end script validation on a microsoft-graph-style operationId.
- **[Reality]** Chaos prometheus counters were registered but never incremented (#436, #79)
  - `mockforge_chaos_faults_total{fault_type, endpoint}`, `mockforge_chaos_latency_ms`, and `mockforge_chaos_rate_limit_violations_total` all existed in the registry, but no caller invoked the corresponding `record_*` methods. `/metrics` reported zero faults regardless of how many were actually firing — masking the effect of configured chaos rules from operators.
  - Wired `record_fault(...)` at every fault decision point in the HTTP middleware (`http_error`, `timeout`, `rate_limit`, `connection_limit`, `packet_loss`, `partial_response`, `payload_corruption`) and the TCP chaos listener (`tcp_reset`, `tcp_close`). Latency injection also now records to the histogram via `record_latency`.
  - The TUI Chaos screen still renders config-only; surfacing these counters as a stats panel is tracked as a follow-up.

## [0.3.127] - 2026-05-08

### Fixed

- **[Reality]** TPS / RPS200 dashboard counters stuck at 0 under load (#351, #79)
  - Root cause: `record_response()` lives inside `collect_http_metrics`, but the middleware was exported from `mockforge-http` and never `.layer()`d onto the production router built in `serve.rs`. CPS kept ticking because `CountingMakeService` wraps the make-service at a different layer.
  - Layer `collect_http_metrics` as the outermost wrapper on `http_app` so every response — including chaos-mutated ones — bumps the rate counters the dashboard sampler reads.
  - Regression test pins the actual counter delta on a 2xx response, not just the response status.

### Added

- **[DevX]** `bench-chunked` accepts `--base-path`, humantime `--duration`, `--validate-requests`, `--export-requests` (#352, #79)
  - `--base-path <PATH>` prepends to every spec-derived operation path before URL construction. CLI > spec.servers > none, matching `mockforge bench`. No-op without `--spec`.
  - `-d 600s` / `--duration <DURATION>` switched from bare seconds (u64) to humantime parsing — `30s`, `5m`, `1h`, or bare seconds all parse.
  - `--validate-requests` (OpenAPI request validation) and `--export-requests` (per-request JSON export) now mirror the same flags on `mockforge bench`.
  - The full Microsoft Graph invocation reported in #79 now parses end-to-end:
    ```
    mockforge bench-chunked --chunk-size-bytes 4096 --total-size-bytes 10485760 \
      --chunk-interval-ms 50 --spec microsoft-graph.yaml \
      --target https://192.168.2.86 --base-path /v1.0 \
      --validate-requests --export-requests --insecure -d 600s
    ```
- **[DevX]** Supervisor wrappers for unattended runs under heavy traffic (#350, #79)
  - `deploy/systemd/mockforge.service` — systemd unit with `Restart=always`, resource limits, and hardening directives. Documents the install dance and the two knobs (`MemoryMax`, `LimitNOFILE`) most likely to matter at high concurrency.
  - `deploy/scripts/run-forever.sh` — bash supervisor that restarts the binary after any non-clean exit. Forwards SIGINT/SIGTERM so Ctrl-C stops cleanly. Useful on macOS / non-systemd hosts and ad-hoc bench rigs.
  - `deploy/systemd/README.md` picks between them.

## [0.3.126] - 2026-05-03

### Added

- **[Observability]** TPS / RPS200 / CPS rate metrics in TUI dashboard + persistent CSV (#326, #79)
  - **TPS** — successful (200..=399) responses per second
  - **RPS200** — 200-OK responses per second
  - **CPS** — accepted TCP connections per second (works for both plain HTTP via `axum::serve` and HTTPS via `axum_server::Server::serve` — make-service wrapper instead of listener wrapper)
  - Each shown current + lifetime peak in the *Request Stats* panel
  - Three new columns appended to the `MOCKFORGE_METRICS_LOG_FILE` CSV: `tps,rps_200,cps`. Old positional CSV parsers keep working
  - New `mockforge-foundation::rate_counters` module hosts the global atomic counters
- **[CLI]** `--https-port` for nginx-style dual HTTP+HTTPS listeners (#327, #79)
  - When set, the existing `--http-port` listener stays plain HTTP and a parallel TLS listener spins up on `--https-port`, sharing the same router and admin UI
  - Example: `mockforge serve --http-port 80 --https-port 443 --tls-cert server.pem --tls-key key.pem`
  - `--https-port` requires `--tls-cert` + `--tls-key` and must differ from `--http-port`
- **[Reality]** `bench-chunked --spec` to drive the native chunked bench from POST/PUT/PATCH operations in an OpenAPI spec (#328, #79)
  - When `--spec` is set, `--target` becomes the base URL and the bench iterates each matching operation sequentially, each running for `--duration`
  - `--operation-id <id>` narrows to a single op
- **[Reality]** `bench-chunked` now captures and prints up to five non-2xx response samples — status, `Server` header, first 256 bytes of body (#329, #79)
  - Critical for diagnosing the "503 from bench, 200 in MockForge log" pattern (almost always an upstream proxy timing out on a slow chunked upload)
  - Hint output for 5xx spells out the proxy-timeout math: each request takes >= `(total_size_bytes / chunk_size_bytes) * chunk_interval_ms` ms

### Changed

- **[Reality]** `bench-chunked` CLI help: `--total-size-bytes` is now explicitly documented as **per-request** (not total over the run), with the chunks-per-request formula in the help text. Resolves repeat user confusion (#328, #79)

## [0.3.125] - 2026-05-02

### Added

- **[Chaos]** Per-request fault matchers (#306, #79):
  - `request_matcher.source_ips` — CIDR or bare IP allowlist
  - `request_matcher.headers` — case-insensitive name + optional exact value
  - `request_matcher.min_body_size_bytes` — only requests with body ≥ N
  - `request_matcher.chunked_only` — only `Transfer-Encoding: chunked` requests
  - AND across fields, OR within a list. Empty matcher matches everything (back-compat)
  - Applies to all five fault paths: HTTP errors, timeouts, partial responses, payload corruption, connection errors
- **[Chaos]** TCP-level connection errors via `ChaosTcpListener` (#306, #79):
  - `connection_error_kind: tcp_reset` — TCP RST at accept time (`SO_LINGER=0` then drop). Clients see `ECONNRESET`
  - `connection_error_kind: tcp_close` — TCP FIN at accept time. Clients see EOF before any HTTP response
  - `connection_error_kind: http_503` (default) — application-layer 503 on a healthy connection (back-compat)
- **[Reality]** `mockforge bench-chunked` — native Rust chunked-encoding traffic generator (#306, #79)
  - Bypasses k6 entirely. Each worker streams body via `reqwest::Body::wrap_stream`, no Content-Length, guaranteed wire chunking
  - Supports `--concurrency`, `--duration`, `--chunk-size-bytes`, `--total-size-bytes`, `--chunk-interval-ms`, `--header`, `--insecure`
- **[TUI]** Peak metrics tracked alongside current values in the dashboard (#306, #79)
  - CPU, memory, error-rate now show `current (peak X)`
- **[Observability]** Persistent metrics CSV via `MOCKFORGE_METRICS_LOG_FILE` env var (#306, #79)
  - 10-second sampling, `timestamp,cpu_pct,mem_mb,total_reqs,err_rate` per row
  - Survives restarts; charts in any spreadsheet, Grafana, or dashboarding tool

### Changed

- **[Chaos]** `timeout_errors: true` now actually `tokio::sleep(timeout_ms)` then returns **504 Gateway Timeout**, applied uniformly to chunked and non-chunked. Previously this flag was incorrectly mapped to body truncation (#306, #79)
- **[Chaos]** `partial_response` now distinguishes chunked vs non-chunked truncation (#306, #79):
  - Non-chunked: truncates body but keeps original `Content-Length` header — clients see unexpected EOF
  - Chunked: truncates before terminating chunk — no `0\r\n\r\n`, real protocol violation
- **[Reality]** k6 templates: `bench --chunked-request-bodies` adds `Transfer-Encoding: chunked` header (best-effort — k6/Go's `net/http` may still send Content-Length based on body type)

### Notes

- Adds new public fields to `FaultInjectionConfig`, `K6Config`, and `K6ScriptTemplateData`. External callers building these via struct literal (without `..Default::default()`) need a trivial update.

## [0.3.124] - 2026-04-30

### Fixed

- **[Core]** `cargo test --doc -p mockforge-core` now passes cleanly — stale doctests against types that moved out during the openapi/foundation extractions are marked `ignore` (#285)
  - Validation release: closes out the chronic CI red on every release tag from v0.3.117 onward

## [0.3.123] - 2026-04-30

### Fixed

- **[UI]** `test_static_assets_content_length` skips its size assertion when no Vite build is present, so `release.yml` (which doesn't run `pnpm`) stops failing (#284)
  - Validation release for the chronic release-CI red

## [0.3.122] - 2026-04-30

### Fixed

- **[UI]** `admin_ui_build` tests skip cleanly when `ui/dist/index.html` is missing (#283)
  - Validation release for the chronic release-CI red

## [0.3.121] - 2026-04-29

### Fixed

- **[Registry]** Drop duplicate migration that blocked all sqlite migrations (#282)
  - `_000010_user_notification_and_preferences.sql` was a stale orphan (byte-identical to `_000011_`) that violated `_sqlx_migrations.UNIQUE(version)` against the legitimate `_000010_federation_scenario_activations.sql`
  - 26 sqlite tests in `mockforge-registry-core` were failing on every release until this landed

## [0.3.120] - 2026-04-29

### Fixed

- **[HTTP]** Mocks created via `POST /__mockforge/api/mocks` now honor `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND` (#281)
  - Previously, OpenAPI-loaded route handlers expanded `{{faker.email}}`, `{{uuid}}`, `{{randInt …}}`, etc., but mocks registered through the management API shipped the literal template strings to clients
  - The Node.js SDK and any test setup that creates mocks programmatically were affected
- **[CI]** `release.yml`'s test step now pre-builds `mockforge` with `--features all-protocols` so e2e tests have a binary that actually starts the WebSocket and gRPC listeners (#281)
- **[Test]** `test_data_protocol_generation` no longer asserts that a JSON-string field magically becomes a JSON number after in-string template expansion (it doesn't, and never has) (#281)

## [0.3.119] - 2026-04-29

### Added

- **[HTTP]** `--no-rate-limit` CLI flag and `MOCKFORGE_RATE_LIMIT_ENABLED=false` env var to fully disable the per-IP HTTP rate limiter (#280, #79)
  - `MOCKFORGE_RATE_LIMIT_DISABLED=true` is a documented alias
  - Reported by @srikr while load-testing the api.github.com spec — sustained load was hitting the default 1000 RPM / 2000 burst limits and returning 429s with `Retry-After: 60`
  - Workaround prior to this release was setting `MOCKFORGE_RATE_LIMIT_RPM=100000000` / `MOCKFORGE_RATE_LIMIT_BURST=100000000`; that still works and is unchanged

## [0.3.104] – [0.3.118] — 2026-03-29 to 2026-04-23

These releases predate the `chore(release): bump workspace to X.Y.Z`
commit convention introduced in 0.3.119, so there's no single commit
to lift detail from per version. Per-release notes are auto-generated
on each GitHub release page:

- <https://github.com/SaaSy-Solutions/mockforge/releases>

The crates are all published and resolvable on crates.io
(`cargo search mockforge-cli` lists every version).

## [0.3.103] - 2026-03-28

### Added

- **[Reality]** Field-level schema validation errors in conformance failure details (#79)
  - Violations now show specific field path, violation type, expected/actual values
  - Uses `jsonschema::validate()` instead of `is_valid()` for detailed error reporting
  - Displayed in terminal output and `conformance-failure-details.json`
- **[Reality]** HAR-to-YAML generator: `mockforge har-to-conformance --har file.har` (#79)
  - Converts browser HAR captures to custom compliance YAML
  - Auto-detects base URL, filters static assets, extracts response headers and JSON body field types
  - Output compatible with `--conformance-custom`
- **[Reality]** Multi-target conformance: `--conformance` + `--targets-file` now works (#79)
  - Runs conformance tests against each target sequentially using native executor
  - Per-target reports in `output/target_N/conformance-report.json`
  - Combined summary in `multi-target-conformance-summary.json`

## [0.3.102] - 2026-03-28

### Fixed

- **[Reality]** Add `summary.json` to spec-driven conformance `handleSummary` (#79)
  - Custom conformance tests (via `--conformance-custom`) now generate `summary.json`

## [0.3.101] - 2026-03-26

### Fixed

- **[Reality]** Skip automatic Authorization (Basic/Bearer) headers when Cookie header is provided via `--conformance-header` (#79)
  - Users managing session-based auth no longer get conflicting Basic Auth headers
- **[Reality]** Add `summary.json` output to reference conformance generator's `handleSummary` (#79)

## [0.3.100] - 2026-03-24

### Fixed

- **[Core]** Prevent panic on conflicting parameter names in OpenAPI routes (#79)
  - Routes with different param names at the same position (e.g., `{attestation_id}` vs `{subject_digest}`) now canonicalize to first-registered names
  - Fixes crashes when loading GitHub, Xero, and similar real-world specs
  - Applied to all 4 router builders
- **[Core]** Swagger 2.0 `formData` parameters now properly converted to OpenAPI 3 `requestBody` (#79)
  - `application/x-www-form-urlencoded` for regular fields, `multipart/form-data` for file uploads

## [0.3.99] - 2026-03-23

### Fixed

- **[Core]** Fix param name conflict panic — same as v0.3.100 (v0.3.99 publish was overwritten by concurrent process)

## [0.3.98] - 2026-03-22

### Fixed

- **[Reality]** Fix zero stats in multi-target summary — removed deprecated `--summary-export` flag; rely solely on `handleSummary()` for writing `summary.json` (#79)
- **[Reality]** Fix failed_requests metric reading `.fails` (success count) instead of `.passes` (failure count) from k6 Rate metric (#79)
- **[Reality]** Fix k6 script path not found when CWD is set to output dir — now uses absolute script path (#79)
- **[Reality]** Fix `--summary-export` absolute path for CWD mismatch (#79)
- **[Reality]** Fix k6 API server port conflict in multi-target mode — each parallel instance gets a unique port (#79)
- **[Reality]** Treat k6 exit code 99 (thresholds crossed) as warning, still parse results (#79)
- **[Reality]** Warn when `--conformance` is used with `--targets-file` (not yet supported) (#79)
- **[Core]** Downgrade contract diff `$ref` resolver warnings from WARN to DEBUG (#79)

### Added

- **[Reality]** Total elapsed time in multi-target summary output and `aggregated_summary.json` (#79)
- **[Reality]** `all_targets.csv` file with per-target metrics for easy parsing (#79)
- **[UI]** Host header column in admin dashboard Recent Logs (#79)
- **[TUI]** Client IP and Host columns in both Dashboard and Logs screens (#79)

## [0.3.91] - 2026-03-17

### Fixed

- **[HTTP]** Fix double-slash in conformance URLs when `--target` has trailing slash (#79)
- **[HTTP]** Apply OData rewrite layer to TLS/HTTPS server path (was only on non-TLS) (#79)
- **[CLI]** Downgrade non-actionable WARN messages to DEBUG/INFO (MockAI key, proto dir, auth backend, JWT secret) (#79)

## [0.3.90] - 2026-03-16

### Added

- **[Core]** OData function call path support via URI rewrite layer (#79)
  - Handles paths like `(period='{period}')` for Microsoft Graph compatibility
  - Mock responses generated for OData function endpoints

## [0.3.89] - 2026-03-15

### Fixed

- **[Core]** Gracefully skip OData function call paths in OpenAPI specs instead of failing (#79)

## [0.3.88] - 2026-03-14

### Added

- **[Reality]** `--conformance-delay` flag to add delay between conformance requests (#79)
- **[Reality]** k6 output logging to file for debugging (#79)
- **[Reality]** 429 rate-limit detection and error clarity in conformance output (#79)

### Fixed

- **[Cloud]** End-to-end deployment pipeline, storage fallback, remaining console errors
- **[CLI]** Load OpenAPI spec from `MOCKFORGE_CONFIG` env var for cloud deployments
- **[Cloud]** Auto-detect Fly.io registry images for cross-app pulls

## [0.3.85] - 2026-03-12

### Added

- **[Reality]** Native conformance executor with API, UI dashboard, and SDK integration (#79)
- **[Reality]** Conformance report UX improvements and custom test authoring (#79)
- **[Reality]** Full request/response detail capture for conformance failures (#79)
- **[Reality]** OWASP API Top 10 coverage mapping in conformance reports (#79)

### Fixed

- **[Reality]** Eliminate misleading error rate in conformance output (#79)
- **[Reality]** Deduplicate native executor checks, write failure details file (#79)
- **[Reality]** Custom conformance checks now emit failure details (#79)
- **[Core/Bench]** Resolve 3 conformance test failures (#79)
- **[UI/Bench]** Fix empty routes and add endpoint details to conformance (#79)

## [0.3.80] - 2026-03-10

### Fixed

- **[Analytics]** Make migrations idempotent with `IF NOT EXISTS` guards (#79)
- **[TUI]** Add g/G and PgUp/PgDn to routes screen status hint (#79)
- **[UI/Bench]** Fix routes proxy, improve OWASP coverage reporting (#79)

### Added

- **[Cloud]** Cloud mode support, auth fixes, runtime error hardening
- **[Cloud]** Deployment deletion CLI command and background cleanup worker
- **[HTTP/UI]** API explorer for hosted mock deployments

### Changed

- **[Refactor]** Architectural overhaul — 10 workstreams across core, UI, CI
- **[Perf]** Replace linear route scan with matchit trie-based matching
- **[Refactor]** Extract chaos modules from core into mockforge-chaos
- **[Refactor]** Split config.rs and openapi/response.rs into submodules
- **[Refactor]** Restrict 8 internal modules to pub(crate), add 35 GraphQL tests
- **[Refactor]** Implement MockProtocolServer trait for all 10 protocols

## [0.3.76] - 2026-03-08

### Fixed

- **[UI/TUI]** Implement 11 missing admin API endpoints that caused "40 Errs" in the TUI dashboard (#79)
  - The TUI polls `/__mockforge/chaos`, `/__mockforge/recorder/status`, `/__mockforge/world-state`, `/__mockforge/federation/peers`, and `/__mockforge/vbr/status` every 2-30 seconds
  - These endpoints previously returned 404, incrementing the TUI error counter ~22 times/minute
  - All 5 GET endpoints now return valid JSON with real data from live subsystem instances
  - 6 POST/DELETE mutation endpoints (`chaos/toggle`, `recorder/start`, `recorder/stop`, `chaos/scenarios/{name}` start/stop) are auth-gated via RBAC
  - Added 5 new handler modules: `chaos_api`, `recorder_api`, `world_state_proxy`, `federation_api`, `vbr_api`
  - 26 E2E tests verified against a real running server (GET responses, auth rejection, auth acceptance, state mutations)
- **[CLI]** Admin server now creates real subsystem instances instead of returning empty defaults (#79)
  - Recorder: created from `--recorder-db` config when `--recorder` flag is passed
  - VBR engine: created with in-memory storage backend (lightweight, no disk side-effects)
  - Federation: empty instance created so the TUI shows a valid (but empty) federation state
- **[TUI]** Error counter capped at 999 to prevent unbounded growth (#79)
- **[CLI]** Fix pre-existing `clippy::print_literal` warning in cloud commands

### Added

- **[Federation]** `Federation::empty()` constructor for creating a default empty federation instance
- **[CLI]** `mockforge-federation` added as a dependency for admin server integration

## [0.3.73] - 2026-03-05

### Fixed

- **[UI]** Fix `cargo publish` failure for `mockforge-ui` caused by `build.rs` modifying source directory
  - Removed code that copied `pwa-manifest.json` and `sw.js` into `ui/dist/` during build (violates cargo's source-dir-immutability rule)
  - `serve_service_worker` now reads `sw.js` from `ui/public/` (same pattern as `serve_manifest`)
  - Added `sw.js` to the crate's `include` list so it's packaged correctly
- **[Core]** Mock server now supports `X-Mockforge-Response-Status` header to return non-default status codes (#79)
  - Conformance checks for `response:404` and `response:400` previously always failed because the server returned the first declared status (usually 200)
  - New `has_response_for_status()` validates the requested code exists in the spec before overriding
  - Both OpenAPI handler paths extract and pass the header through
- **[Core]** Response generation no longer replaces object-typed properties with string examples (#79)
  - When a property schema declares `type: object`, the fallback now preserves an empty `{}` instead of generating a name-based string like `"example config"`
  - Fixes `response:schema:validation` failures where JSON schema validation rejected string values for object properties
  - Added `is_object_typed_property()` helper for type-aware fallback decisions
- **[Data]** `generate_by_type("object")` now returns `{}` instead of `"unknown_type_object"` (#79)
  - Also added `"array"` handler returning `[]`

### Changed

- **[Reality]** Spec-driven conformance generator now sends `X-Mockforge-Response-Status` header for `response:400` and `response:404` checks (#79)
  - Tells the mock server which status code to return, enabling accurate status-code conformance testing

## [0.3.72] - 2026-03-04

### Fixed

- **[UI]** `mockforge serve --admin` no longer panics when no production auth is configured (#79)
  - `validate_auth_config_on_startup()` now logs a warning instead of returning an error
  - Auto-generated JWT secret fallback so the admin UI works out of the box
  - Default users are seeded even without `ENVIRONMENT=development`, so login works immediately
- **[Reality]** Fix duplicate session ID in conformance Cookie headers (#79)
  - Removed invalid `noCookies: true` from k6 options (not a real k6 option; k6 silently ignores it)
  - Added `http.cookieJar().clear(BASE_URL)` before and after each request when custom Cookie headers are present
  - Prevents k6's internal cookie jar from re-sending server `Set-Cookie` values alongside custom headers
  - Applied to both reference-mode (`generator.rs`) and spec-driven (`spec_driven.rs`) generators
- **[Reality]** Fix missing single-quote escaping in spec-driven `format_headers()` (#79)
  - Header values containing single quotes are now properly escaped, matching `generator.rs` behavior

### Added

- **[Reality]** Conformance report now shows individual failed checks with pass/fail counts (#79)
  - New "Failed Checks" section after the category summary table lists each check that failed
  - When not using `--conformance-all-operations`, prints a tip suggesting it for endpoint-level detail

## [0.3.70] - 2026-02-27

### Fixed

- **[Reality]** Remove dead `CUSTOM_HEADERS` JS const from conformance generators (#79)
  - Custom header values are now inlined directly into each request instead of referencing an unused JS constant
  - Eliminates confusing dead code in generated k6 scripts
- **[Reality]** Add `noCookies: true` to k6 options when Cookie header is in custom headers (#79)
  - Prevents k6's automatic cookie jar from duplicating cookies on subsequent requests
  - Fixes duplicate session ID / authentication failures reported by @srikr
- **[Reality]** Fix conformance report file not found after k6 execution (#79)
  - `handleSummary` now writes `conformance-report.json` to an absolute path matching the output directory
  - Previously wrote to a relative path based on k6's CWD, causing the CLI to report "Conformance report not generated"

### Added

- **[Reality]** `--conformance-all-operations` flag for full-endpoint conformance testing (#79)
  - Default mode tests one representative operation per feature check (fast feature-coverage)
  - New flag tests ALL operations with path-qualified check names (e.g., `method:GET:/api/users`)
  - Addresses user confusion about "only 5 endpoints tested"
- **[Reality]** Conformance coverage summary output (#79)
  - After generating conformance tests, prints "Conformance: N operations analyzed, M unique checks generated"
  - When using default mode with fewer checks than operations, shows tip about `--conformance-all-operations`

## [0.3.69] - 2026-02-24

### Fixed

- **[Multi]** Replace 36+ `assert!(true)` placeholder tests with meaningful assertions across 16 files
  - CLI command tests (MQTT, SMTP, governance) now construct and verify command variants
  - Registry server tests use compile-time type checks instead of no-op assertions
  - Integration tests (voice workspace, drift GitOps, behavioral cloning, WebSocket, cross-platform sync) use proper verification patterns
- **[gRPC]** Add `use super::*` to 13 empty `test_module_compiles()` tests so they actually verify module compilation
- **[HTTP]** Fix misleading "placeholder" doc comment on fully-implemented `get_proxy_inspect` handler

## [0.3.57] - 2026-02-14

### Fixed

- **[Reality]** Spec-driven conformance: global security requirement detection (#79)
  - `annotate_security()` now falls back to `spec.security` (root-level) when an operation has no operation-level security defined
  - APIs that only define security globally are now correctly detected
- **[Reality]** Spec-driven conformance: SecurityScheme type resolution (#79)
  - Security schemes are now resolved from `components.securitySchemes` to detect actual type (`HTTP/bearer`, `APIKey`, `HTTP/basic`) instead of relying on name heuristics alone
  - A scheme named "myAuth" that is actually an `apiKey` type is now correctly identified
  - Name-based heuristic retained as fallback for unresolvable schemes
- **[Reality]** Spec-driven conformance: ContentNegotiation detection (#79)
  - `ContentNegotiation` feature is now detected when a response defines multiple content types (e.g., both `application/json` and `application/xml`)
  - Previously only worked in reference mode
- **[Reality]** CLI help text for `--conformance-categories` now includes `response-validation` (#79)

### Added

- **[Reality]** 5 new conformance tests: ResponseValidation with schema check, global security, SecurityScheme resolution, ContentNegotiation detection, single-type negative case (#79)

## [0.3.56] - 2026-02-14

### Added

- **[Reality]** Conformance category filtering (#79)
  - New `--conformance-categories` flag to run only specific conformance categories (e.g., `--conformance-categories "parameters,security"`)
  - Case-insensitive category matching with validation against known categories
- **[Reality]** Spec-driven conformance testing (#79)
  - When `--conformance --spec my-api.json` is provided, analyzes the user's actual OpenAPI spec to detect which features their API exercises
  - Generates conformance tests against real endpoints instead of reference `/conformance/` paths
  - Full `$ref` resolution with cycle detection for parameters, schemas, request bodies, and responses
  - Detects: parameter types, request body formats, schema types/composition/formats/constraints, response codes, security schemes
- **[Reality]** Response schema validation (#79)
  - In spec-driven mode, validates response bodies against OpenAPI response schemas
  - `SchemaValidatorGenerator` produces JavaScript validation expressions from OpenAPI schemas
  - Supports object (required fields, property types), array, string (format regex, enum, length), integer/number (range), boolean validation
  - Wrapped in try-catch for resilient k6 execution
- **[Reality]** SARIF 2.1.0 report output (#79)
  - New `--conformance-report-format sarif` flag outputs conformance results in SARIF 2.1.0 format
  - Compatible with GitHub Code Scanning, VS Code SARIF Viewer, and CI/CD pipelines
  - Maps each conformance feature to a SARIF rule with OpenAPI spec section links
  - Passed features emit `level: "note"`, failed features emit `level: "error"`

## [0.3.55] - 2026-02-14

### Added

- **[Reality]** Per-server stats in multi-target mode (#79)
  - `K6Results` now parses RPS, VUs, and full latency breakdown (min/med/p90/p95/p99/max) from k6 `summary.json`
  - `AggregatedMetrics` includes `total_rps`, `avg_rps`, `total_vus_max`
  - Multi-target reporter shows per-target RPS, VUs, and full latency breakdown
  - `aggregated_summary.json` includes all new metrics in both aggregated and per-target sections
- **[Reality]** Per-target spec support for multi-target mode (#79)
  - Targets file JSON format now supports `"spec"` field for per-target OpenAPI specs
  - Each target can use a different spec file for heterogeneous fan-out
  - Example: `[{"url": "https://server1", "spec": "spec_a.json"}, {"url": "https://server2", "spec": "spec_b.json"}]`
- **[Reality]** OpenAPI 3.0.0 conformance testing (#79)
  - New `--conformance` flag generates and runs comprehensive k6 scripts exercising 47 OpenAPI 3.0.0 features across 10 categories (Parameters, Request Bodies, Schema Types, Composition, String Formats, Constraints, Response Codes, HTTP Methods, Content Negotiation, Security)
  - Reports per-category pass/fail rates with colored terminal output
  - Supports `--conformance-api-key`, `--conformance-basic-auth`, `--conformance-report` for security scheme testing
  - Example: `mockforge bench --conformance --target http://localhost:3000`

## [0.3.54] - 2026-02-13

### Fixed

- **[Reality]** fix(bench): deliver CRS payloads as path injection + form-encoded body (#79)
  - Added `inject_as_path` field to `SecurityPayload` — URI payloads without query params (e.g., CRS 942101: `POST /1234%20OR%201=1`) now replace the request path via `encodeURI()` so WAFs inspect `REQUEST_FILENAME` instead of `ARGS`
  - Added `form_encoded_body` field to `SecurityPayload` — body payloads from CRS tests (e.g., 942432: `var=;;dd foo bar`) now sent as `application/x-www-form-urlencoded` so WAFs parse form data into `ARGS` for character counting
  - Updated `k6_script.hbs` and `k6_crud_flow.hbs` templates to handle both new delivery mechanisms
  - Replaced unreliable `startsWith('/')` URI heuristic in CRUD flow template with explicit `injectAsPath` flag
  - Expected SQLi detection: 46/46 rules (100%), up from 45/46 (97.8%)

## [0.3.53] - 2026-02-13

### Fixed

- **[Reality]** fix(bench): URL-encode URI payloads + strip form keys from body payloads (#79)
  - URI security payloads now wrapped in `encodeURIComponent()` for valid HTTP transport — WAFs decode before inspection (fixes 942101)
  - Form-encoded body payloads now have form key prefix stripped (`var=;;dd foo bar` → `;;dd foo bar`) so WAF ARGS parsing sees the attack payload directly (fixes 942432)
  - Confirmed SQLi detection: 45/46 rules (97.8%), up from 43/46 (93.5%)

## [0.3.52] - 2026-02-12

### Fixed

- **[Reality]** fix(bench): Group multi-part WAFBench payloads + decode body payloads + fix Cookie/CookieJar conflict (#79)
  - Multi-part CRS test cases (URI + headers + body) now grouped by `group_id` and sent together in one HTTP request instead of being split across separate requests (fixes 942290)
  - Body payloads from CRS YAML files are now form-URL-decoded before injection (`%22+WAITFOR+DELAY+%27` → `" WAITFOR DELAY '`) so WAFs see actual SQL patterns in JSON bodies (fixes 942240, 942320, 942432)
  - URI payloads from path-only CRS tests are now URL-decoded and stripped of leading `/` artifact (fixes 942101)
  - Cookie header payloads no longer overridden by empty CookieJar — `secRequestOpts` conditionally skips `jar: new http.CookieJar()` when a security Cookie header is present (fixes 942420, 942421)
  - Added `groupedPayloads` array-of-arrays in generated k6 scripts; `getNextSecurityPayload()` returns arrays of related payloads
  - Template loop applies URI/header/body parts simultaneously per request via `secPayloadGroup`
  - Expected SQLi detection improvement: 37/46 → 44/46 (80.4% → 95.7%)

## [0.3.51] - 2026-02-11

### Fixed

- **[Reality]** fix(bench): Accept all WAFBench CRS payloads without attack-pattern filter (#79)
  - Removed overly strict `attack-pattern` category filter that was silently dropping valid CRS test cases
  - All CRS YAML test cases now loaded regardless of their `attack_type` metadata

## [0.3.50] - 2026-02-10

### Fixed

- **[Reality]** fix(bench): Use per-request CookieJar instead of shared EMPTY_JAR (#79)
  - Each HTTP request now creates its own `new http.CookieJar()` instead of sharing a global empty jar
  - Prevents cookie cross-contamination between requests in security testing

## [0.3.49] - 2026-02-09

### Fixed

- **[Reality]** fix(bench): Send raw security payloads + use dedicated empty cookie jar (#79)
  - Security payloads now sent as raw strings without additional encoding
  - Dedicated empty CookieJar per request prevents k6's default cookie accumulation

## [0.3.48] - 2026-02-08

### Fixed

- **[Reality]** fix(bench): Cycle security payloads per-operation + clear cookies in API2 tests (#79)
  - Security payloads now cycle per-operation block (each API endpoint gets a different payload)
  - Previously all operations in one VU iteration used the same payload
  - OWASP API2 (Broken Auth) tests now properly clear cookies between requests

## [0.3.47] - 2026-02-06

### Added

- **[DevX]** chore: Add Claude Code setup (CLAUDE.md, agents, skills, hooks, hookify)
  - Project-specific Claude Code configuration with rules, agents, and skills
  - Custom skills for verification, template checking, code review, and bench review
  - Hookify rules engine for behavioral guardrails

### Fixed

- **[Reality]** fix(bench): Security payloads now injected + cookie dedup in all templates (#79)
  - Security payloads now properly injected in both k6_script.hbs and k6_crud_flow.hbs templates
  - Cookie deduplication applied to all HTTP request paths in both templates
  - Comprehensive test suite added for issue #79 security pipeline

- **[Registry]** fix(registry): Add RBAC permission system with Display, AdminAll bypass, and PermissionChecker
  - New RBAC permission model with role-based access control
  - AdminAll role bypasses all permission checks
  - PermissionChecker trait for consistent authorization across endpoints

## [0.3.46] - 2026-01-30

### Fixed

- **[Reality]** fix(bench): WAFBench payloads now distributed across VUs for better coverage (#79)
  - Changed payload cycling to use VU-based offset: `(__VU - 1) % payloads.length`
  - Previously all 50 VUs started at index 0 and cycled through same sequence
  - Now each VU starts at a different payload, maximizing attack coverage in shorter test runs
  - With 50 VUs and 30 payloads, all payloads are tested from the start

- **[Reality]** fix(bench): OWASP API tests now include custom headers in all requests (#79)
  - Added `CUSTOM_HEADERS` to API8 verbose error test (malformed JSON body test)
  - Added `CUSTOM_HEADERS` to API9 discovery paths test
  - Added `CUSTOM_HEADERS` to API9 API versions test
  - Fixes auth failures when using `--headers "Cookie:..."` with OWASP testing

## [0.3.43] - 2026-01-16

### Fixed

- **[Reality]** fix(bench): Security payloads now actually applied to requests in k6 scripts (#79)
  - Updated k6_script.hbs template to call `getNextSecurityPayload()` and `applySecurityPayload()`
  - Previously, security payload functions were defined but never called in generated scripts
  - Security payloads now properly injected into request bodies for POST/PUT/PATCH
  - Header-based payloads now properly injected into request headers

## [0.3.42] - 2026-01-15

### Fixed

- **[Reality]** fix(bench): XSS payloads now inject into ALL string fields, not just the first one (#79)
  - Removed `break` statement from `applySecurityPayload()` loop in security_payloads.rs
  - Ensures WAF can detect payloads regardless of which field it scans
- **[Reality]** fix(bench): Added `jar: null` to remaining OWASP HTTP calls to prevent cookie duplication (#79)
  - Fixed testBrokenAuth empty token test
  - Fixed testMisconfiguration verbose error test
  - Fixed testInventory discovery paths and API versions checks
- **[CLI]** fix(cli): Fixed format string compilation error in plugin_commands.rs (#79)
  - Escaped all braces (`{` → `{{`, `}` → `}}`) inside `format!` macro for auth plugin template
  - Fixes "invalid format string: expected `}`, found `r`" compilation error

## [0.3.39] - 2026-01-14

### Fixed

- **[Reality]** fix(bench): WAFBench XSS attacks now properly injected into request body (#79)
  - Removed location check from `applySecurityPayload()` - ALL payloads now injected into body for POST/PUT
  - WAFBench payloads correctly pass location info (uri/header/body) to k6 scripts
  - Header payloads include header name for proper injection into specified headers
- **[Reality]** fix(bench): Cookie header duplication in OWASP and security tests (#79)
  - Added `jar: null` to all HTTP request params to disable k6's automatic cookie jar
  - Prevents duplicate cookies when user provides Cookie header via `--headers` flag
  - Applied to k6_script.hbs, k6_crud_flow.hbs, and OWASP generator

## [0.3.38] - 2026-01-13

### Fixed

- **[Reality]** fix(bench): pass custom headers from `--headers` flag to OWASP tests (#79)
  - Cookie and other custom headers are now included in all OWASP request helpers
  - Fixes issue where `avi-sessionid=None` was being sent instead of actual cookie values
- **[Reality]** fix(bench): WAFBench loader now handles single YAML file paths (#79)
  - Previously only directories or glob patterns were supported
  - Single file paths like `/path/to/941100.yaml` now work correctly
- **[Reality]** Verified CRS v3.3 format compatibility with full CoreRuleSet test suite
  - Tested with 175 files, 1512 payloads (692 XSS, 505 SQLi, 304 Command Injection, 11 Path Traversal)

## [0.3.37] - 2026-01-12

### Added

- **[Reality]** feat(bench): add WAFBench cycle-all mode (`--wafbench-cycle-all`) to test all payloads sequentially (#79)
- **[Reality]** feat(bench): add `--owasp-iterations` parameter to control OWASP test iterations per VU (#79)
- **[Reality]** feat(bench): OWASP tests now respect `--vus` parameter for concurrent testing (#79)

### Fixed

- **[Reality]** fix(bench): WAFBench payloads now properly injected in standard bench mode (not just CRUD flow)
- **[Reality]** fix(bench): OWASP APIs now use random UUIDs per request instead of static IDs for BOLA testing (#79)
- **[Reality]** fix(bench): OWASP auth tokens with special characters (quotes, backslashes) now properly escaped (#79)
- **[Reality]** fix(bench): prevent Handlebars double-escaping of pre-escaped JavaScript values
- **[Reality]** fix(bench): WAFBench security payloads now integrated into CRUD flow requests (#79)
- **[Reality]** fix(owasp): use `http.del()` instead of `http.delete()` for k6 compatibility (#79)
- **[Reality]** fix(owasp): add `--base-path` support for OWASP API testing (#79)
- **[Reality]** fix(bench): remove undefined `totalRequestCount` variable reference
- **[Reality]** fix(bench): support CRS v3.3 WAFBench format and pass `--insecure` to OWASP tests

## [0.3.33] - 2026-01-10

### Fixed

- **[Reality]** fix(bench): multiple fixes for OWASP and WAFBench testing
  - Support CRS v3.3 format in WAFBench parser
  - Pass `--insecure` flag to OWASP tests for self-signed certificates

## [0.3.31] - 2026-01-08

### Fixed

- **[Reality]** fix(bench): fix extracted value substitution in CRUD flows
- **[Reality]** fix(bench): OWASP k6 configuration improvements

## [0.3.30] - 2026-01-07

### Added

- **[Reality]** feat(bench): add `merge_body` support for CRUD flows - merge extracted values with request body
- **[Reality]** feat(bench): add `inject_attacks` data model for security testing in CRUD flows

## [0.3.28] - 2026-01-06

### Added

- **[Reality]** feat(bench): add nested path extraction for CRUD flows (e.g., `results[0].id`)
- **[Reality]** feat(bench): add filter extraction for CRUD flows (e.g., `results[?name=='test'].id`)

## [0.3.27] - 2026-01-05

### Added

- **[Reality]** feat(bench): add full body extraction for CRUD flows
- **[Reality]** feat(bench): add key filtering for extracted values

## [0.3.26] - 2026-01-04

### Added

- **[Reality]** feat(bench): add aliased extraction for CRUD flow value chaining
  - Extract values with aliases (e.g., `id as poolId`) for use in subsequent requests

## [0.3.24] - 2026-01-03

### Fixed

- **[Reality]** fix(bench): use correct variable name in CRUD flow extracted value replacement

## [0.3.22] - 2026-01-02

### Added

- **[Reality]** feat(bench): add OWASP API Security Top 10 testing mode (#79)
  - Test for BOLA (API1), Broken Auth (API2), Mass Assignment (API3), Resource Consumption (API4)
  - Test for Function Auth (API5), SSRF (API7), Misconfiguration (API8), Inventory (API9), Unsafe Consumption (API10)
  - Configurable test categories with `--owasp-categories`
  - Support for auth tokens with `--owasp-auth-token`
  - SARIF and JSON report formats

### Changed

- **[DevX]** chore: include UI dist files for publishing to crates.io

## [0.3.21] - 2025-12-31

### Fixed

- **[DevX]** fix(bench): use custom flow config and fix sequential mode path matching - enables cross-resource dependency chains
- **[DevX]** fix(bench): process dynamic placeholders in CRUD flow params file bodies (#79)
- chore: update benchmark baseline [skip ci]
- chore: enable publishing for previously internal crates
- chore: update benchmark baseline [skip ci]
- fix(release): disable sccache for crates.io publish
- chore: update benchmark baseline [skip ci]
- fix(release): publish all crates in dependency order
- fix(release): add mockforge-core to crates.io publish order
- chore: update benchmark baseline [skip ci]
- feat(bench): add --base-path option for API base path support (#79)
- chore: update benchmark baseline [skip ci]
- fix(collab): include SQLx query cache for crates.io installation (#79)
- chore: update benchmark baseline [skip ci]
- feat: implement optional enhancements from improvement plan
- fix: update doc tests to use rust,ignore for external dependencies
- chore: update benchmark baseline [skip ci]
- chore: add missing crates to workspace and restore path dependencies
- chore: restore path dependencies after publishing remaining v0.3.17 crates
- fix: restore all crates to workspace members list
- chore: restore path dependencies after publishing v0.3.17
- docs: update CHANGELOG for v0.3.17 release
- feat(bench): add WAFBench YAML integration for security testing
- Bump version to 0.3.17
- feat: comprehensive improvements across AMQP, MQTT, gRPC, registry server, and UI
- feat(ui): add type safety, mobile layout fixes, and search/filter to frontend
- Restore path dependencies after publishing v0.3.16
- Bump version to 0.3.16
- fix: resolve flaky tests and race conditions across test suite
- fix: replace panic-prone unwrap calls with safe error handling
- fix: resolve UUID storage format mismatch in collab crate tests
- Add multi-spec support and cross-spec dependency detection for bench command
- feat: add multi-spec support and cross-spec dependency handling to bench command
- fix: add validation to CRUD flow script generation
- fix: sanitize k6 CRUD flow metric names (#79 follow-up)
- Bump version to 0.3.13 and improve changelog
- Bump version to 0.3.12 and publish to crates.io
- Bump version to 0.3.11 and publish to crates.io
- chore: update benchmark baseline [skip ci]
- feat: add --params-file option for custom parameter values in bench
- Bump version to 0.3.10 and publish to crates.io
- chore: update benchmark baseline [skip ci]
- fix: move insecureSkipTLSVerify to global k6 options (fixes --insecure)
- chore: update benchmark baseline [skip ci]
- fix: resolve k6 bench issues with --insecure flag, textSummary, and query params
- chore: update benchmark baseline [skip ci]
- chore: bump version to 0.3.9 and update changelog
- feat: implement comprehensive mock server functionality across all crates
- chore: commit remaining version updates
- fix: enable publishing for mockforge-ui
- fix: enable publishing for mockforge-tunnel
- fix: update all 0.3.7 dependencies to 0.3.8 with path dependencies
- fix: add path dependencies for all workspace crates
- chore: update CHANGELOG date for 0.3.8
- chore: bump version to 0.3.8
- Fix cargo publish issues: add version requirements to dependencies
- chore: update benchmark baseline [skip ci]
- Apply formatting and additional code changes
- Fix compilation errors: update dependencies and adapt to API changes
- fix: remove path from mockforge-pipelines dep in mockforge-collab
- Add mockforge-sdk, mockforge-ui, mockforge-cli to workspace
- fix: add mockforge to restore function targets list
- fix: convert mockforge dev-dependencies to path dependencies
- fix: add mockforge-core to restore list and manually fix dependency
- fix: include mockforge-core in restore list
- fix: restore function now properly handles table-form dependencies without path
- fix: automatically restore dependencies at start of publish
- fix: restore all crate dependencies, not just a few
- fix: only convert dependencies for already-published crates
- fix: correct publish order - publish mockforge-data before mockforge-core
- fix: add mockforge-data as optional dependency in mockforge-core
- chore: bump version to 0.3.6 and update changelog
- chore: update benchmark baseline [skip ci]
- Fix k6 script generation and UI icon embedding issues
- chore: update benchmark baseline [skip ci]
- Add comprehensive test suite and fix build issues
- chore: update benchmark baseline [skip ci]
- docs: add comprehensive performance benchmarks documentation
- chore: update benchmark baseline [skip ci]
- fix: implement real functionality in benchmark tests and fix k8s-operator
- chore: update benchmark baseline [skip ci]
- fix: filter out 'change' directories from benchmark baseline parsing
- chore: update benchmark baseline [skip ci]
- chore: update benchmark baseline [skip ci]
- fix: GitHub Actions workflow cleanup and fixes (#81)
- chore: restore dependencies after publishing all crates
- fix: add mockforge-cli to workspace and add metadata to mockforge-k8s-operator
- fix: add missing crates to workspace (mockforge-sdk, mockforge-http, mockforge-ui, mockforge-k8s-operator)
- fix: add mockforge-world-state to workspace and publishing order before mockforge-http
- fix: add mockforge-route-chaos publishing step before mockforge-http
- fix: add mockforge-route-chaos to dependency targets and publishing order
- fix: add mockforge-route-chaos to workspace and publishing script
- fix: add mockforge-route-chaos to publishing order before mockforge-http
- fix: reduce keywords from 6 to 5 for mockforge-performance
- fix: reduce keywords to 5 for mockforge-performance (crates.io limit)
- fix: add mockforge-performance to publishing order before mockforge-http
- fix: add mockforge-collab to workspace members list
- fix: add mockforge-collab to workspace members
- fix: add missing README.md for mockforge-pipelines
- fix: add mockforge-pipelines to publishing order and dependency targets
- fix: add mockforge-pipelines to workspace and publishing script
- fix: add all missing crates to workspace members
- fix: handle short form dependencies when converting to path
- fix: publish mockforge-template-expansion before mockforge-core
- fix: add mockforge-template-expansion to publishing script
- fix: temporarily convert dependent crates' dependencies to path before publishing
- fix: remove argon2 from mockforge-core during MSRV checks
- fix: exclude mockforge-collab from MSRV checks and remove patch section
- fix: use awk instead of sed for multi-line patch section insertion
- fix: use Cargo patch section to pin base64ct for MSRV
- fix: improve base64ct pinning order in MSRV workflow
- fix: use exact version constraint for base64ct in MSRV workflow
- fix: improve base64ct pinning in MSRV workflow
- fix: pin base64ct to 1.7 for MSRV compatibility
- fix: exclude mockforge-ui from MSRV checks
- fix: add abd and existant to typos config
- fix: exclude FontAwesome and all minified files from spell check
- fix: also remove sysinfo from mockforge-ui during MSRV checks
- fix: exclude elasticlunr.min.js from spell check
- fix: exclude highlight.js from spell check
- fix: disable sysinfo feature during MSRV checks
- fix: sync sysinfo to 0.37, fix resolvable typo, exclude ace.js from spell check
- fix: pin sysinfo to 0.36, fix typos, improve MSRV workaround
- fix: update MSRV to 1.80 and add GraphQL exclusion workaround
- fix: update MSRV from 1.82 to 1.75
- fix: fix GitHub Actions workflow failures
- fix: standardize dependencies and fix all test failures
- Skip CRDs in kubectl validation to avoid server connection
- Fix kubectl validation to prevent server connection attempts
- Fix kubectl validation to skip server connection
- Fix all test failures and resolve dependency conflicts
- Fix k6 metric name validation error (issue #79) (#80)
- Optimize workflows: update deprecated actions and add path filters
- Fix mockforge-smtp version constraint from 0.2.0 to 0.3.3
- Fix Docker build, k8s validation, and spell check issues
- fix: update all mockforge dependency versions to 0.3.3 in mockforge-http
- chore: fix formatting (pre-commit hooks)
- deps(deps): bump opentelemetry_sdk from 0.21.2 to 0.31.0 (#67)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump opentelemetry-semantic-conventions (#66)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump sysinfo from 0.32.1 to 0.37.2 (#60)
- deps(deps): bump wasmparser from 0.239.0 to 0.240.0 (#64)
- deps(deps): bump governor from 0.6.3 to 0.8.1 (#61)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump mail-parser from 0.9.4 to 0.11.1 (#63)
- deps(deps): bump rumqttc from 0.24.0 to 0.25.0 (#65)
- deps(deps): bump ndarray from 0.16.1 to 0.17.1 (#76)
- chore: update benchmark baseline [skip ci]
- ci(deps): bump azure/setup-helm from 3 to 4 (#72)
- ci(deps): bump actions/upload-artifact from 4 to 5 (#71)
- deps(deps): bump image from 0.24.9 to 0.25.9 (#77)
- deps(deps): bump rustls from 0.21.12 to 0.23.35 (#78)
- chore: update benchmark baseline [skip ci]
- Bump all crates to version 0.3.3
- Format code with rustfmt
- Fix k6 script generation with operation IDs containing dots/hyphens
- chore: update benchmark baseline [skip ci]
- perf: optimize template rendering by avoiding unnecessary operations
- chore: update benchmark baseline [skip ci]
- docs: update benchmark documentation with final optimizations
- perf: fix benchmark regressions and optimize measurements
- chore: update benchmark baseline [skip ci]
- Fix Kafka compilation errors and borrow checker issues
- feat: Implement cross-pillar enhancements - World State Engine, MOD, and Performance Mode
- feat(ai-studio): Add API Critique, System Generator, and Behavioral Simulator
- chore: rework UI/UX to be more AI native
- fix: Address pre-commit security vulnerabilities
- feat: Implement Invisible Mock Server experience (DevX Pillar)
- feat(security): implement email, Slack, and webhook notification services
- Refactor template expansion for Send safety
- chore: Restore path dependencies after 0.3.2 publish
- Fix: Complete SQLx query cache for mockforge-collab 0.3.2
- chore: update mockforge dependencies to version 0.3.1 across multiple crates
- fix: improve dependency conversion for optional dependencies and fix publishing order
- fix: update publish script to handle Phase 1 crate dependencies correctly
- feat: add comprehensive integration tests for 0.3.0 features and update changelog
- feat: Complete pillar enhancement gaps - VS Code extension and docs
- feat: Implement pillar tagging system and documentation enhancements
- feat: Implement MockForge AI Studio - Unified AI Copilot
- feat(cloud): Complete Cloud pillar implementation and fix compilation issues
- [DevX] Add JSON Schema support for config validation and IDE autocompletion
- feat: Implement Contract Fitness Functions, Consumer Impact Analysis, and Multi-Protocol Contracts
- feat: Enhance Reality feature with observability, cross-protocol consistency, and time-aware lifecycles
- fix: use proper vosk API by matching on CompleteResult enum
- fix: resolve all compilation errors
- chore: prepare release 0.3.0
- feat: Implement LLM Studio - Natural Language Workspace Creation (0.3.4)
- feat: Complete Behavioral Cloning v1 implementation and refactor architecture
- feat: Implement Drift Budget & GitOps for API Sync + AI Contract Diff
- feat: implement Scenario Studio Visual Editor with React Flow
- feat: implement AI-Native Interface Deepening features
- feat: Implement Time Travel & Snapshots and Frontend X-Ray Mode
- feat(sdk): Add Contract-Backed Types and Scenario-First SDKs to Vue, Svelte, and Angular
- Format code: Apply rustfmt and whitespace cleanup
- Release v0.2.9: Update version, CHANGELOG, and publish all crates to crates.io
- Add registry server improvements, password reset, metrics, and marketplace enhancements
- security: Upgrade wasmtime to 36.0.3 to fix RUSTSEC-2025-0118
- feat: Fix compilation errors and implement comprehensive E2E test suite
- fix: implement custom routes, template expansion, latency injection, and init improvements
- feat: Smart Personas with array generation and relationship inference
- feat: Complete Java and .NET SDK implementations with builder patterns
- fix: update all test files for new function signatures
- fix: resolve all compilation errors across workspace
- Complete Phase 3 security controls implementation
- Add cloud monetization infrastructure and features
- Implement organization management endpoints
- Fix Axum 0.8 route syntax in state_machine_api.rs
- Fix file server route syntax for Axum 0.8 compatibility
- Release v0.2.8: Publish all crates to crates.io
- chore: bump version to 0.2.8
- feat: Complete Generative Schema Mode and achieve 100% roadmap completion
- Implement Smart Personas feature for consistent cross-endpoint data generation
- Add Reality Continuum feature for blending mock and real data sources
- Implement Voice + LLM Interface with STT backends
- Implement complete Deceptive Deploy feature
- Add GraphQL + REST Playground with workspace filtering
- Implement ForgeConnect SDK with full feature set
- Add enhanced scenario marketplace features
- Configure SQLx and integrate mockforge-collab with mockforge-core
- Fix test compilation errors in reality integration and hot-reload tests
- Implement Reality Slider feature with hot-reload support
- Complete latency recording integration and fix WorkspaceConfig reality_level field
- style: Apply rustfmt formatting to Chaos Lab code
- feat: Add Chaos Lab interactive network condition simulation
- Fix test compilation errors in openapi_generator_tests
- Fix all compilation errors for AI Contract Diff feature
- Add WireMock-inspired features: browser proxy mode, git sync, data sources, template library, managed hosting docs, and user management
- Add comprehensive ecosystem and use cases documentation
- Complete configuration and extensibility implementation
- Add advanced behavior and simulation features
- Fix test and benchmark compilation errors
- Complete Scenario State Machines 2.0 with sub-scenario execution
- Implement VBR Engine enhancements: OpenAPI integration, M2M relationships, seeding, ID generation, snapshots
- Add mock-to-real migration pipeline with per-route toggling
- Add Data Scenarios Marketplace feature
- feat: Implement ForgeConnect - Front-End Integrated Mode for browser-based mock creation
- Add MockForge Cloud Graph visualization with real-time updates and export
- Add data personality profiles system for consistent mock data generation
- Add realistic network conditions and chaos lab with interactive UI controls
- Add temporal simulation with CLI commands and scenario support
- Complete MockAI implementation with query params and session recording
- Add Virtual Backend Reality (VBR) engine
- Add multipart form data support and file generation/serving for API mocks
- fix: update mockforge-plugin-sdk to use workspace version
- fix: enable publishing for mockforge-tunnel and add to publish script

## [0.3.20] - 2025-12-31

### Fixed

- **[Bench] Dynamic placeholder expansion in CRUD flow params file bodies** (#79): Fixed `${__VU}`, `${__ITER}`, and other dynamic placeholders not being expanded when used in request body content from params files
  - Previously, placeholders like `"name": "HTTP-WAAP-vsvip-${__VU}-${__ITER}"` were sent literally to the API
  - Now properly converted to k6 template literals for runtime evaluation
  - Supports all dynamic placeholders: `${__VU}`, `${__ITER}`, `${__TIMESTAMP}`, `${__UUID}`, `${__RANDOM}`, `${__COUNTER}`, `${__DATE}`, `${__VU_ITER}`

## [0.3.19] - 2025-12-30

### Added

- **[DevX] API base path support for bench command** (#79): New `--base-path` option to prepend a path prefix to all API endpoints in generated load tests
  - Automatically extracts base path from OpenAPI spec's `servers` URL (e.g., `https://api.example.com/api/v1` → `/api/v1`)
  - CLI option takes priority over spec's base path for explicit control
  - Use `--base-path ""` to disable base path even if spec defines one
  - Works with both standard k6 scripts and CRUD flow mode
  - Example usage:
    ```bash
    # Auto-detect from spec's servers URL
    mockforge bench --spec api.yaml --target http://localhost:8080 --crud-flow

    # Explicitly set base path
    mockforge bench --spec api.yaml --target http://localhost:8080 --base-path /api

    # Disable base path
    mockforge bench --spec api.yaml --target http://localhost:8080 --base-path ""
    ```

## [0.3.18] - 2025-12-29

### Fixed

- **[Collab] SQLx offline mode for crates.io installation** (#79): Fixed compilation errors when installing `mockforge-collab` from crates.io
  - Added `.sqlx` query cache directory with 51 precompiled query metadata files
  - The `build.rs` now automatically enables `SQLX_OFFLINE=true` when query cache is present
  - Users no longer need `DATABASE_URL` or to run `cargo sqlx prepare` to install the crate
  - Resolves "set DATABASE_URL to use query macros online" compilation errors

## [0.3.17] - 2025-12-28

### Added

- **[DevX] WAFBench YAML integration for security testing**: New `--wafbench-dir` flag to import Microsoft WAFBench CRS (Core Rule Set) attack patterns
  - Parse WAFBench YAML test files from the [WAFBench project](https://github.com/microsoft/WAFBench)
  - Support glob patterns for loading specific rule categories (e.g., `REQUEST-941-*` for XSS, `REQUEST-942-*` for SQLi)
  - Extract attack payloads from URI parameters, headers, and request bodies
  - Automatic CRS rule ID parsing from test metadata (e.g., `941100` for XSS attacks)
  - Integrate WAFBench payloads with existing security testing framework
  - Example usage:
    ```bash
    mockforge bench spec.yaml --wafbench-dir ./wafbench/REQUEST-941-*  # XSS rules
    mockforge bench spec.yaml --wafbench-dir ./wafbench/**/*.yaml      # All rules
    ```

- **[DevX] Per-URI control mode for data-driven testing** (#79): New `--per-uri-control` flag for CSV/JSON data files that allows each row to specify HTTP method, URI, body, query params, headers, attack type, and expected status code
  - Enables fine-grained control over test requests directly from data files
  - Supports security testing per-URI with `attack_type` column
  - Automatic status validation with `expected_status` column
  - Example CSV format:
    ```csv
    method,uri,body,query_params,headers,attack_type,expected_status
    GET,/virtualservice,,include_name=true,,,200
    POST,/virtualservice,"{""name"":""test""}",,,sqli,201
    ```

- **[Protocol] AMQP TLS support**: Full TLS/SSL support for AMQP broker with configurable certificates
- **[Protocol] MQTT protocol improvements**: Enhanced MQTT server with TLS, session management, and metrics
- **[Protocol] gRPC dynamic service improvements**: Better dynamic proto loading and error handling
- **[Registry] Security enhancements**: CSRF protection, request ID middleware, trusted proxy support, token revocation
- **[UI] Frontend improvements**: Type safety fixes, mobile layout improvements, search/filter functionality

### Changed

- Comprehensive dependency updates across workspace crates

### Fixed

- **[DevX] CRUD flow params file integration** (#79): Fixed `--params-file` not being applied in CRUD flow mode
  - Body configurations from params file are now correctly applied to POST/PUT/PATCH operations in `--crud-flow` mode
  - Fixed body serialization issue that caused "ReferenceError: object is not defined" error in generated k6 scripts
  - Body is now properly serialized as a JSON string for the Handlebars template
- **[Core] Race conditions and flaky tests**: Resolved timing issues across test suite
- **[Core] Panic-prone unwrap calls**: Replaced with safe error handling throughout codebase

## [0.3.16] - 2025-12-27

### Added

- Version bump with dependency updates

### Fixed

- **[Test] Flaky test fixes**: Resolved race conditions and timing issues in integration tests
- **[Core] Safe error handling**: Replaced panic-prone `.unwrap()` calls with proper error handling

## [0.3.15] - 2025-12-26

### Added

- **[DevX] Multi-spec support for bench command**: The `mockforge bench` command now supports loading and merging multiple OpenAPI specifications
  - Multiple `--spec` flags: `mockforge bench --spec pools.yaml --spec vs.yaml --target https://api.com`
  - Directory discovery with `--spec-dir`: `mockforge bench --spec-dir ./specs/ --target https://api.com`
  - Conflict resolution strategies with `--merge-conflicts`: `error` (default), `first`, `last`
  - Spec mode selection with `--spec-mode`: `merge` (default) combines all specs, `sequential` runs specs in dependency order
  - Sequential execution mode with per-spec output directories and results
  - Leverages existing multi-spec infrastructure from mockforge-core
- **[DevX] Cross-spec dependency detection**: New `spec_dependencies` module for handling dependencies between specs
  - Automatic detection of dependencies from field naming patterns (`pool_ref`, `pool_id`, `poolId`, etc.)
  - Schema registry for cross-referencing schemas across multiple specs
  - Topological sorting for correct execution order
  - Manual dependency configuration via `--dependency-config` (YAML/JSON)
  - Support for value extraction and injection between spec groups

### Changed

- `BenchCommand.spec` field changed from `PathBuf` to `Vec<PathBuf>` to support multiple specs
- `SpecParser` now includes `from_spec()` method for pre-loaded OpenAPI specs
- Added `dependency_config` field to `BenchCommand` for cross-spec value passing configuration

### Fixed

- Nothing yet.

## [0.3.14] - 2025-12-26

### Added

- Version bump to 0.3.14

### Changed

- Nothing yet.

### Fixed

- Nothing yet.

## [0.3.13] - 2025-12-24

### Fixed

- **[DevX] k6 CRUD flow metric name sanitization** (#79 follow-up): Fixed invalid k6 metric names in CRUD flow scripts when flow names contain dots or special characters
  - CRUD flow names are now sanitized for use as k6 metric names (e.g., `plans.list` → `plans_list`)
  - Original flow names preserved in comments and group names for readability
  - Made `sanitize_js_identifier` function public for reuse across k6 generators
  - Added script validation to CRUD flow generation for defense in depth

## [0.3.12] - 2025-12-23

### Changed

- **[DevX] Dependency updates**: Version alignment and dependency updates across all workspace crates

## [0.3.11] - 2025-12-19

### Added

- **[DevX] Custom benchmark parameters**: Added `--params-file` option to `mockforge bench` command for loading custom parameter values from a file

  **Why it matters**: Allows users to define reusable parameter configurations for benchmark runs, making it easier to test different scenarios without modifying command-line arguments each time.

## [0.3.10] - 2025-12-18

### Fixed

- **[DevX] k6 benchmark script generation fixes**: Resolved multiple issues with generated k6 scripts
  - Fixed `--insecure` flag handling by moving `insecureSkipTLSVerify` to global k6 options
  - Fixed `textSummary` import and usage in generated scripts
  - Fixed query parameter encoding in benchmark requests

## [0.3.9] - 2025-12-17

### Added

- **[Reality] Comprehensive Mock Server Implementation**: Full implementation across all protocol crates
  - **mockforge-amqp**: Complete AMQP 0-9-1 broker with exchanges, queues, bindings, messages, protocol handling, fixtures, and spec registry
  - **mockforge-kafka**: Full Kafka broker with consumer groups, partitions, topics, metrics, and protocol handling
  - **mockforge-mqtt**: Complete MQTT broker with QoS levels, topic subscriptions, and retained messages
  - **mockforge-ftp**: Virtual filesystem, spec registry, and fixture support
  - **mockforge-smtp**: Email server with fixtures and spec registry
  - **mockforge-tcp**: TCP server with fixtures and protocol support
  - **mockforge-grpc**: Dynamic proto parser, service generator, reflection, and metrics
  - **mockforge-graphql**: Full handler implementations

- **[DevX] Enhanced CLI Commands**: New commands for all protocols and features
  - AMQP, Kafka, MQTT, FTP, SMTP protocol commands
  - Blueprint, cloud, deploy, dev-setup, governance commands
  - Logs, progress, recorder, scenario, snapshot commands
  - Time manipulation, VBR, voice, wizard, and workspace commands
  - AI-powered mock generation commands

- **[Reality] Virtual Backend Repository (VBR)**: Complete data management system
  - API generator, entity management, constraints, and validation
  - Database integration with migrations and schema management
  - Session handling, snapshots, and mutation rules
  - ID generation strategies and scheduling

- **[Reality] World State Engine**: Coherent world simulation
  - State engine with model and query support
  - Entity relationships and lifecycle management

- **[AI] Enhanced AI Capabilities**: AI-powered mock generation
  - RAG-based AI response generator
  - AI event generator for WebSocket scenarios
  - Behavioral cloning with scenario types

- **[Cloud] Collaboration Features**: Team collaboration support
  - Backup, merge, and promotion workflows
  - Multi-environment configuration
  - Client SDK improvements

- **[DevX] Observability & Analytics**: Enhanced monitoring
  - Pillar usage tracking and analytics queries
  - Metrics middleware and coverage tracking
  - Latency metrics and performance monitoring

- **[Contracts] Chaos Engineering**: Resilience testing capabilities
  - Failure designer and incident replay
  - Chaos API with configurable fault injection
  - Route-level chaos with latency distributions

- **[DevX] Plugin System Enhancements**: Extended plugin capabilities
  - Backend generator and datasource support
  - Runtime adapter improvements
  - SDK builders and testing utilities

- **[Cloud] Registry Server**: Complete registry implementation
  - Authentication, authorization, and RBAC
  - Redis caching, email notifications
  - Organization and subscription models
  - API token management and audit logging

- **[DevX] UI Server**: Dashboard and admin features
  - Admin handlers for workspace management
  - Chain visualization and coverage metrics
  - Failure analysis and promotion workflows
  - Graph visualization and health monitoring

## [0.3.8] - 2025-01-27

### Fixed

- **[DevX] Compilation errors resolved**: Fixed all compilation errors across the workspace
  - Updated `axum-server` from 0.6 to 0.8 with `tls-rustls-no-provider` feature
  - Updated `rustls` from 0.21 to 0.23, `rustls-pemfile` from 1.0 to 2.0, `tokio-rustls` from 0.24 to 0.26
  - Adapted TLS code to rustls 0.23 API (CertificateDer, PrivateKeyDer, WebPkiClientVerifier)
  - Fixed multi_spec module: properly exported and resolved compilation errors
  - Fixed handle_serve function calls: added missing parameters and fixed type mismatches
  - Fixed borrow checker issues in multi_spec merging logic
  - Added missing documentation for enum variants and struct fields
  - Fixed various type mismatches and iteration patterns

- **[DevX] Cargo publish readiness**: Fixed all dependency version requirements for crates.io publishing
  - Added version requirements to all path dependencies in mockforge-cli, mockforge-chaos, mockforge-http, mockforge-route-chaos, mockforge-vbr
  - Set `publish = false` for desktop-app and tests packages (not meant for crates.io)
  - All crates now pass `cargo publish --dry-run` validation

## [0.3.6] - 2025-11-25

### Fixed

- **[DevX] k6 script generation with operation IDs containing dots/hyphens** (#79)
  - Fixed "Unexpected token ." error when OpenAPI operation IDs contain dots (e.g., `plans.create`) or hyphens (e.g., `plans.update-pricing-schemes`)
  - Changed `is_alphanumeric()` to `is_ascii_alphanumeric()` in JavaScript identifier sanitization to ensure ASCII-only identifiers
  - All operations are now properly included in generated k6 scripts with valid JavaScript identifiers
  - Added comprehensive tests including integration test with full billing subscriptions spec

- **[DevX] UI icon embedding for published crates**
  - Fixed build failures when installing `mockforge-cli` from crates.io due to missing icon files
  - Updated `build.rs` to read icon files at build time and embed them as byte array literals
  - Replaced `include_bytes!` with `CARGO_MANIFEST_DIR` approach that failed in published crates
  - Icons are now properly embedded and work both in development and when installing from crates.io

## [0.3.0] - 2025-11-17

### Added

- **[DevX] Pillars & Tagged Changelog**: Complete pillar system implementation with documentation and tooling
  - Defined five foundational pillars: [Reality], [Contracts], [DevX], [Cloud], [AI]
  - Added comprehensive PILLARS.md documentation with feature mappings
  - Implemented CI validation for pillar tags in changelog entries
  - Added pillar tagging instructions to release tooling
  - Updated README and getting-started guide with pillars section

  **Why it matters**: Clear product story spine that makes it obvious what each release invests in. Pillar tags help users understand product direction and find features relevant to their needs.

- **[Reality] Smart Personas & Reality Continuum v2**: Complete persona graph and lifecycle system
  - Persona graphs with relationship linking across entities
  - Lifecycle states (NewSignup, Active, PowerUser, ChurnRisk, Churned, etc.)
  - Reality Continuum integration with field-level and entity-level mixing
  - Fidelity score calculation and API endpoint
  - Comprehensive PERSONAS.md documentation

  **Why it matters**: Upgrade from "random-but-consistent fake data" to "coherent world simulation." Personas maintain relationships across endpoints, and fidelity scores quantify how real your mock environment is.

- **[Contracts] Drift Budget & GitOps for API Sync**: Complete drift management system
  - Hierarchical drift budget configuration (global, workspace, service, endpoint)
  - Breaking change detection and classification
  - Incident management with webhook integration
  - GitOps PR generation for contract updates
  - Comprehensive DRIFT_BUDGETS.md documentation

  **Why it matters**: Make MockForge the "drift nerve center" for contracts. Define acceptable drift, get alerts when budgets are exceeded, and automatically generate PRs to update contracts and fixtures.

- **[Reality] Behavioral Cloning v1**: Multi-step flow recording and replay
  - Flow recording with request/response capture and timing
  - Flow viewer with timeline visualization
  - Scenario replay engine with strict/flex modes
  - Scenario storage and export/import (YAML/JSON)
  - Comprehensive BEHAVIORAL_CLONING.md documentation

  **Why it matters**: Move from endpoint-level mocks to journey-level simulations. Record realistic flows from real systems and replay them as named scenarios for comprehensive testing.

- **[AI][DevX] LLM/Voice Interface for Workspace Creation**: Natural language to complete workspace
  - Natural language workspace creation from descriptions
  - Automatic persona and relationship generation
  - Behavioral scenario generation (happy path, failure, slow path)
  - Reality continuum and drift budget configuration from NL
  - Voice and text input support
  - Comprehensive LLM Studio documentation

  **Why it matters**: The golden path: "Describe the system in natural language → MockForge builds a realistic mock backend with personas, behaviors, and reality level config." No manual configuration required.

- **[DevX] Comprehensive Integration Test Coverage**: Complete test suite for all 0.3.0 features
  - Smart Personas v2 integration tests (15 tests covering persona graphs, lifecycle states, fidelity scores)
  - Drift Budget integration tests (14 tests covering budget hierarchy, breaking change detection, incident management)
  - Drift GitOps integration tests (16 tests covering PR generation, OpenAPI/fixture updates, GitOps configuration)
  - Behavioral Cloning integration tests (15 tests covering flow recording, scenario replay, strict/flex modes)
  - Voice/LLM Workspace Creation integration tests (16 tests covering command parsing, workspace building, NL to workspace flow)
  - All tests passing with 100% success rate (76 total integration tests)

  **Why it matters**: Production-ready features require production-ready tests. Comprehensive integration test coverage ensures reliability, prevents regressions, and provides confidence for users adopting these features.

### Changed

- Changelog entries now require pillar tags for all major features
- Release process includes automated pillar tag validation
- Documentation structure updated to highlight pillars

### Fixed

- Nothing yet.

### Security

- Nothing yet.

## [0.2.9] - 2025-11-14

### Added

- **[Cloud] Registry server improvements** with password reset functionality

  **Why it matters**: Enable seamless team collaboration with secure registry access—teams can share and discover mock scenarios without friction, and password reset keeps workflows moving when credentials are lost.

- **[Cloud] Enhanced metrics and marketplace features**
- **[DevX] Comprehensive E2E test suite**
- **[DevX] Custom routes implementation**
- **[Reality] Template expansion improvements**
- **[Reality] Latency injection enhancements**
- **[Reality] Smart Personas** with array generation and relationship inference

  **Why it matters**: Generate realistic, interconnected mock data automatically—arrays that make sense, relationships that stay consistent across endpoints, and personas that feel like real users without manual configuration.

- **[DevX] Complete Java and .NET SDK implementations** with builder patterns

  **Why it matters**: Bring MockForge to enterprise teams using Java and .NET—no more language barriers, no more custom integration work. Your entire stack can use the same mock infrastructure.

- **[Cloud] Cloud monetization infrastructure and features**

  **Why it matters**: Enable sustainable platform growth with flexible pricing models—teams can scale from free tier to enterprise without friction, and the platform can grow while serving developers.

- **[Cloud] Organization management endpoints**

  **Why it matters**: Scale from solo developer to enterprise team—manage users, permissions, and resources at the org level, not just individual accounts. Real teams need real organization tools.

- **[Cloud] Security controls implementation** (Phase 3)

  **Why it matters**: Protect production deployments with enterprise-grade security—fine-grained access controls, audit trails, and compliance features that let you trust MockForge with sensitive data and critical workflows.

### Changed

- **[DevX] Upgraded wasmtime to 36.0.3** to fix RUSTSEC-2025-0118
- **[DevX] Fixed Axum 0.8 route syntax compatibility** across multiple modules
- **[DevX] Updated all test files** for new function signatures

### Fixed

- **[DevX] Fixed compilation errors** across workspace
- **[DevX] Fixed Axum 0.8 route syntax** in state_machine_api.rs
- **[DevX] Fixed file server route syntax** for Axum 0.8 compatibility
- **[DevX] Resolved all compilation errors** for comprehensive test coverage

### Security

- **[DevX] Upgraded wasmtime to 36.0.3** to address RUSTSEC-2025-0118
- **[Cloud] Completed Phase 3 security controls implementation**

## [0.2.8] - 2025-11-10

### Added

- **[Reality] Generative Schema Mode**: Complete implementation of generative schema mode for dynamic mock data generation

  **Why it matters**: Spin up a believable API even when the backend doesn't exist yet—no sample DB or seed data required.

- **[Reality] Smart Personas**: Feature for consistent cross-endpoint data generation using persona-based templates

- **[Reality] Reality Continuum**: Feature for blending mock and real data sources with configurable reality levels

  **Why it matters**: Turn the dial between deterministic mock and noisy production-like chaos without changing your client code.

- **[Reality] Reality Slider**: Hot-reload support for reality level adjustments

  **Why it matters**: Adjust reality levels on the fly during development and testing without restarting the server.

- **[Reality] Chaos Lab**: Interactive network condition simulation tool

  **Why it matters**: Test how your application handles real-world network conditions like latency spikes, packet loss, and connection failures.

- **[Contracts] AI Contract Diff**: Feature for comparing and diffing API contracts

  **Why it matters**: Automatically detect and visualize API contract changes to catch breaking changes before they reach production.

- **[DevX] Voice + LLM Interface**: Voice interface implementation with Speech-to-Text (STT) backend support

- **[Reality] Deceptive Deploy**: Complete deceptive deploy feature for advanced testing scenarios

- **[DevX] GraphQL + REST Playground**: Interactive playground with workspace filtering capabilities

- **[DevX] ForgeConnect SDK**: Complete SDK implementation with full feature set

- **[Cloud] Enhanced Scenario Marketplace**: Improved scenario marketplace with additional features

- **[DevX] WireMock-Inspired Features**: Browser proxy mode, git sync, data sources, template library, managed hosting documentation, and user management

- **[DevX] Ecosystem Documentation**: Comprehensive ecosystem and use cases documentation

- **[DevX] Configuration Extensibility**: Complete configuration and extensibility implementation

- **[Reality] Advanced Behavior Simulation**: Enhanced behavior and simulation features

### Changed

- **[DevX] SQLx Integration**: Configured SQLx and integrated mockforge-collab with mockforge-core
- **[Reality] Latency Recording**: Completed latency recording integration with WorkspaceConfig reality_level field support

### Fixed

- **[DevX] Fixed test compilation errors** in reality integration and hot-reload tests
- **[DevX] Fixed test compilation errors** in openapi_generator_tests
- **[Contracts][DevX] Fixed all compilation errors** for AI Contract Diff feature
- **[DevX] Applied rustfmt formatting** to Chaos Lab code

### Security

- Nothing yet.

## [0.2.7] - 2025-11-05

### Added

- **[Contracts] Automatic API Sync & Change Detection**: Implemented periodic polling and automatic sync for detecting upstream API changes

  **Why it matters**: Keep your mocks in sync with real APIs automatically—catch breaking changes before they break your tests.

  - Periodic sync service with configurable intervals (default: 1 hour)
  - Automatic change detection using deep response comparison (status, headers, body)
  - Optional automatic fixture updates when changes detected
  - Manual sync trigger via API (`POST /api/recorder/sync/now`)
  - Sync status tracking and change history
  - Configurable sync settings: upstream URL, interval, headers, timeout, max requests
  - Support for GET-only or all-methods sync
  - Detailed change reports with before/after comparisons
  - Database update method for refreshing recorded responses
  - API endpoints: `/api/recorder/sync/status`, `/api/recorder/sync/config`, `/api/recorder/sync/changes`

- **[Reality] TCP Protocol Support**: Added raw TCP server mocking support via new `mockforge-tcp` crate

  **Why it matters**: Mock any protocol that runs over TCP—not just HTTP. Perfect for testing database clients, custom protocols, and legacy systems.

  - Raw TCP connection handling with fixture-based matching
  - Echo mode for testing TCP clients
  - TLS/SSL support for encrypted connections
  - Delimiter-based message framing (optional)
  - Configurable buffer sizes and connection limits
  - CLI flag `--tcp-port` for custom TCP server port
  - Configuration via `config.tcp` in YAML/JSON config files

- **[Reality] Response Selection Modes**: Added support for sequential (round-robin) and random response selection when multiple examples are available
  - Sequential mode: Cycles through available examples in order (round-robin)
  - Random mode: Randomly selects from available examples
  - Weighted random mode: Random selection with custom weights per example
  - Configuration via `x-mockforge-response-selection` OpenAPI extension
  - Environment variable support: `MOCKFORGE_RESPONSE_SELECTION_MODE` (global) and `MOCKFORGE_RESPONSE_SELECTION_<OPERATION_ID>` (per-operation)
  - State tracking for sequential mode ensures round-robin behavior across requests

- **[Reality] Webhook HTTP Execution**: Implemented actual HTTP request execution in chaos orchestration hooks
  - `HookAction::HttpRequest` now executes real outbound HTTP requests (previously only logged)
  - Supports GET, POST, PUT, DELETE, PATCH methods
  - Configurable request body and headers
  - Error handling and logging for webhook failures
  - Fire-and-forget execution (failures don't block orchestration)

- **[DevX] CRUD & Webhook Documentation**: Added comprehensive documentation guides
  - `docs/CRUD_SIMULATION.md`: Complete guide for simulating CRUD operations with stateful data store
  - `docs/WEBHOOKS_CALLBACKS.md`: Full documentation of webhook capabilities via hooks, chains, and scripts
  - Examples demonstrating realistic workflows and integrations

### Changed

- Nothing yet.

### Deprecated

- Nothing yet.

### Removed

- Nothing yet.

### Fixed

- Nothing yet.

### Security

- Nothing yet.

## [0.2.6] - 2025-11-04

### Added

- **[DevX] TLS/HTTPS and mTLS Support**: Added TLS/HTTPS and mutual TLS (mTLS) support for HTTP server
  - Configurable TLS certificate and key paths
  - Client certificate authentication support
  - Secure connection handling for production deployments

- **[DevX] Built-in Tunneling Service**: Added built-in tunneling service for exposing local servers via public URLs
  - Automatic tunnel creation for local development
  - Public URL generation for testing and demos
  - Integration with popular tunneling services

- **[DevX] SDK Implementation**: Completed Phase 1 & 2 of SDK implementation
  - Comprehensive documentation and examples
  - Production-ready client generators

### Changed

- **[DevX] Version Bumps**: Updated all workspace crates from 0.2.5 to 0.2.6
  - Updated all dependency versions across the workspace
  - Fixed version mismatches in mockforge-ui and mockforge-plugin-loader

- **[DevX] Publishing Improvements**: Enhanced crate publishing process
  - Added mockforge-tcp and mockforge-test to publish script
  - Enabled publishing for mockforge-test crate
  - Fixed mockforge-tcp to remove README requirement

### Fixed

- **[DevX] Documentation**: Fixed missing module-level documentation in test files
  - Added comprehensive module documentation to all test modules
  - Improved code documentation consistency

- **[DevX] Axum Compatibility**: Fixed Axum 0.8 compatibility issues in proxy server module
  - Updated proxy server to work with latest Axum version
  - Resolved breaking changes from Axum upgrade

- **[Reality] MQTT Error Types**: Fixed MQTT publish handlers error types to be Send + Sync
  - Updated error types for proper async/await compatibility
  - Ensured thread-safety in MQTT handlers

## [0.2.5] - 2025-01-27

### Added

- **[DevX] OAuth2 Flow Support**: Complete OAuth2 implementation with all standard flows
  - Authorization Code flow with PKCE (RFC 7636 compliant, SHA256 hash)
  - Client Credentials flow for server-side applications
  - Password flow for trusted clients
  - Implicit flow support
  - Automatic token refresh and expiration management
  - State parameter for CSRF protection
  - PKCE code verifier/challenge generation helpers
  - Token storage with expiration tracking (localStorage)

- **[DevX] Enterprise Error Handling**: Structured error handling for generated clients
  - `ApiError` class with status codes, statusText, and error body
  - `RequiredError` class for missing required fields
  - Helper methods: `isClientError()`, `isServerError()`, `getErrorDetails()`, `getVerboseMessage()`
  - Optional verbose error messages with detailed validation information

- **[Contracts] Request/Response Validation**: Built-in validation support
  - Required field validation before sending requests
  - Basic response structure validation (type checking, object validation)
  - Configurable via `validateRequests` flag
  - Detailed validation error messages

- **[DevX] Request/Response Interceptors**: Custom request/response/error transformation
  - Request interceptor: Modify requests before sending
  - Response interceptor: Transform responses after receiving
  - Error interceptor: Global error handling
  - Support for async interceptors

- **[DevX] Enhanced Authentication**: Multiple authentication methods
  - Bearer token (static or dynamic function)
  - API key authentication (static or dynamic)
  - Basic authentication (username/password)
  - OAuth2 (all flows, takes priority over other methods)

- **[DevX] PKCE Helper Functions**: Exported utilities for PKCE implementation
  - `generatePKCECodeVerifier()`: Generate cryptographically random code verifier
  - `generatePKCECodeChallenge()`: Generate SHA256 code challenge from verifier

- **[DevX] Security Best Practices**: Comprehensive security warnings and guidance
  - Client secret warnings for browser-based applications
  - XSS vulnerability warnings for localStorage token storage
  - CSRF protection via state parameter validation
  - Token expiration checking
  - Security documentation in generated README

- **[DevX] Request Timeout Handling**: Configurable request timeouts
  - Default 30-second timeout (configurable)
  - AbortController-based timeout implementation
  - Proper timeout error handling

- **[DevX] React Query Integration Documentation**: Comprehensive examples for @tanstack/react-query integration

### Changed

- **[DevX] React Client Generator**: Major enhancements to generated React client code
  - Replaced placeholder PKCE implementation with full SHA256-based solution
  - Implemented proper response validation (previously placeholder)
  - Enhanced README with comprehensive feature documentation
  - Improved error messages and validation details
  - Better security documentation and best practices

- **[DevX] Operation ID Sanitization**: Improved identifier generation
  - Enhanced `sanitize_identifier` function to handle complex operation IDs
  - Better handling of parentheses, slashes, hyphens in operation IDs
  - Proper camelCase conversion with word boundary detection

### Fixed

- **[DevX] TypeScript Empty Object Types**: Fixed formatting issue where empty object schemas generated invalid TypeScript
  - Empty objects now correctly generate as `[key: string]: any;` instead of malformed `Record<string, any>}`

- **[DevX] DELETE Operations with Query Params**: Fixed missing query parameter support in DELETE operations

- **[DevX] Duplicate Operation IDs**: Fixed duplicate operation ID handling by appending numeric suffixes

- **[DevX] PKCE Code Challenge**: Fixed PKCE implementation to use proper SHA256 hash instead of plain encoding

- **[Contracts][DevX] Response Validation**: Replaced placeholder with actual implementation (type checking, structure validation)

### Security

- **[DevX] Added comprehensive security warnings** for OAuth2 client secrets in browser code
- **[DevX] Added XSS vulnerability warnings** for localStorage token storage
- **[DevX] Implemented CSRF protection** via state parameter validation
- **[DevX] Added token expiration checking** to prevent use of expired tokens
- **[DevX] Documented security best practices** in generated client README

## [0.2.4] - 2025-01-27

### Fixed

- **[DevX] Fix request body parameter generation** in React/Vue/Svelte client generators - request bodies now correctly generate `data` parameter and `body: JSON.stringify(data)` in API client methods
- **[DevX] Fix required vs optional field handling** in generated TypeScript interfaces - required fields no longer incorrectly marked with optional marker (`?`)
- **[DevX] Fix OpenAPI serde deserialization** by adding `#[serde(rename)]` attributes for `operationId` and `requestBody` fields
- **[DevX] Apply required fields processing consistently** across all client generators (React, Vue, Svelte)

### Added

- **[DevX] Comprehensive test coverage** for request body parameter scenarios (POST, PUT, PATCH, DELETE)
- **[DevX] Test cases for `$ref` schemas** in request bodies
- **[DevX] Test cases for YAML spec support** verification

## [0.2.3] - 2025-01-27

### Fixed

- **[DevX] Fix OpenAPI example extraction** to prioritize explicit examples from schema and properties
- **[DevX] Fix request body parameter generation** in React client generator for POST, PUT, PATCH, DELETE methods
- **[DevX] Fix Handlebars template logic** for request body type generation in client code
- **[DevX] Fix useCallback dependency array formatting** in React hooks template
- **[DevX] Add comprehensive test coverage** for request body parameter scenarios

## [0.2.0] - 2025-10-29

### Added

- **[DevX] Output control features** for MockForge generator with comprehensive configuration options
- **[DevX] Unified spec parser** with enhanced validation and error reporting
- **[DevX] Multi-framework client generation** with Angular and Svelte support
- **[Reality] Enhanced mock data generation** with OpenAPI support
- **[DevX] Configuration file support** for mock generation
- **[DevX] Browser mobile proxy mode** implementation
- **[DevX] Comprehensive documentation** and example workflows

### Changed

- **[DevX] Enhanced CLI** with progress indicators, error handling, and code quality improvements
- **[DevX] Comprehensive plugin architecture documentation**

### Fixed

- **[DevX] Remove tests that access private fields** in mock data tests
- **[DevX] Fix compilation issues** in mockforge-collab and mockforge-ui
- **[DevX] Update mockforge-plugin-core version** to 0.1.6 in plugin-sdk
- **[DevX] Enable SQLx offline mode** for mockforge-collab publishing
- **[DevX] Add description field** to mockforge-analytics
- **[DevX] Add version requirements** to all mockforge path dependencies
- **[DevX] Fix publish order dependencies** (mockforge-chaos before mockforge-reporting)
- **[DevX] Update Cargo.lock** and format client generator tests

## [0.1.3] - 2025-10-22

### Changes

- **[DevX] docs: prepare release 0.1.3**
- **[DevX] docs: update CHANGELOG for 0.1.3 release**
- **[DevX] docs: add roadmap completion summary**
- **[DevX] feat: add Kubernetes-style health endpoint aliases and dashboard shortcut**
- **[DevX] feat: add unified config & profiles with multi-format support**
- **[Reality] feat: add capture scrubbing and deterministic replay**
- **[DevX] feat: add native GraphQL operation handlers with advanced features**
- **[Reality] feat: add programmable WebSocket handlers**
- **[Reality] feat: add HTTP scenario switching for OpenAPI response examples**
- **[DevX] feat: add mockforge-test crate and integration testing examples**
- **[DevX] build: enable publishing for mockforge-ui and mockforge-cli**
- **[DevX] build: extend publish script for internal crates**
- **[DevX] build: parameterize publish script with workspace version**

## [0.1.2] - 2025-10-17

### Changes

- **[DevX] build: make version update tolerant**
- **[DevX] build: manage version references via wrapper**
- **[DevX] build: mark example crates as non-publishable**
- **[DevX] build: drop publish-order for cargo-release 0.25**
- **[DevX] build: centralize release metadata in release.toml**
- **[DevX] build: remove per-crate release metadata**
- **[DevX] build: fix release metadata field name**
- **[DevX] build: move workspace release metadata into Cargo.toml**
- **[DevX] build: require execute flag for release wrapper**
- **[DevX] build: automate changelog generation during release**
- **[DevX] build: add release wrapper with changelog guard**
- **[DevX] build: align release tooling with cargo-release 0.25**

## [0.1.1] - 2025-10-17

### Added

- **[Contracts] OpenAPI request validation** (path/query/header/cookie/body) with deep $ref resolution and composite schemas (oneOf/anyOf/allOf).
- **[Contracts] Validation modes**: `disabled`, `warn`, `enforce`, with aggregate error reporting and detailed error objects.
- **[DevX] Runtime Admin UI panel** to view/toggle validation mode and per-route overrides; Admin API endpoint `/__mockforge/validation`.
- **[DevX] CLI flags and config options** to control validation (including `skip_admin_validation` and per-route `validation_overrides`).
- **[DevX] New e2e tests** for 2xx/422 request validation and response example expansion across HTTP routes.
- **[DevX] Templating reference docs** and examples; WS templating tests and demo update.
- **[Reality] Initial release of MockForge** - Multi-protocol mocking framework
- **[Reality] HTTP API mocking** with OpenAPI support
- **[Reality] gRPC service mocking** with Protocol Buffers
- **[Reality] WebSocket connection mocking** with replay functionality
- **[DevX] CLI tool** for easy local development
- **[DevX] Admin UI** for managing mock servers
- **[DevX] Comprehensive documentation** with mdBook
- **[DevX] GitHub Actions CI/CD pipeline**
- **[DevX] Security audit integration**
- **[DevX] Pre-commit hooks** for code quality

### Changed

- **[Contracts] HTTP handlers now perform request validation** before routing; invalid requests return 400 with structured details (when `enforce`).
- **[Contracts] Bump `jsonschema` to 0.33** and adapt validator API; enable draft selection and format checks internally.
- **[Contracts] Improve route registry and OpenAPI parameter parsing**, including styles/explode and array coercion for query/header/cookie parameters.

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- **[DevX] Resolve admin mount prefix** from config and exclude admin routes from validation when configured.
- **[Contracts] Various small correctness fixes** in OpenAPI schema mapping and parameter handling; clearer error messages.

### Security

- N/A

---

## Release Process

This project uses [cargo-release](https://github.com/crate-ci/cargo-release) for automated releases.

### Creating a Release

1. **Patch Release** (bug fixes):

   ```bash
   make release-patch
   ```

2. **Minor Release** (new features):

   ```bash
   make release-minor
   ```

3. **Major Release** (breaking changes):

   ```bash
   make release-major
   ```

### Manual Release Process

If you need to do a manual release:

1. Update version in `Cargo.toml` files
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -m "chore: release vX.Y.Z"`
4. Tag: `git tag vX.Y.Z`
5. Push: `git push && git push --tags`
6. Publish to crates.io: `cargo publish`

### Pre-release Checklist

- [ ] All tests pass (`make test`)
- [ ] Code formatted (`make fmt`)
- [ ] Lints pass (`make clippy`)
- [ ] Security audit passes (`make audit`)
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Version bumped in all `Cargo.toml` files
- [ ] Breaking changes documented (if any)
- [ ] CI passes on all branches

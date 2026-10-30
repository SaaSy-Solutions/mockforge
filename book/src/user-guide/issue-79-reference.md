# Issue #79: offline discussion and CVE traffic labs

This is a companion to [Srikanth's latest question in issue #79](https://github.com/SaaSy-Solutions/mockforge/issues/79#issuecomment-6367266823).
It covers saving the conversation, the requested CVEs, and what a MockForge
traffic test can establish. Advisory references were checked on **2026-10-24**.

## Save every comment and search offline

From a checkout of `main`, with Python 3.11+:

```bash
python3 scripts/export_github_issue.py --repo SaaSy-Solutions/mockforge \
  --issue 79 --output-dir issue-79-export
```

The command reads the issue body and **all comment pages**, including threads
longer than GitHub's first page. It produces:

| File | Use |
| --- | --- |
| `discussion.html` | Open locally, search with Ctrl+F / Cmd+F, print and choose **Save as PDF**. |
| `discussion.md` | Searchable text containing all bodies, code blocks and logs. |
| `snapshot.json` | Original API records, author handles, creation/edit times and source URLs. |

HTML displays bodies as escaped Markdown source. It keeps tables and long logs
complete without running HTML or scripts from a comment, fetching images, or
requiring an internet connection to search. PDF creation is a browser action;
the exporter itself does not install a PDF converter. For an offline re-render:

```bash
python3 scripts/export_github_issue.py --input issue-79-export/snapshot.json \
  --repo SaaSy-Solutions/mockforge --issue 79 --output-dir issue-79-another-copy
```

Use a new output directory each time. A fetch failure or inconsistent comment
count fails instead of silently creating a partial discussion. The export is a
point-in-time copy, not an atomic GitHub backup: later edits and deletions are
not recoverable. Reactions, timeline events, linked PR discussions, and binary
attachment contents are outside the export; their original URLs remain in the
comment text. If `gh` is installed, the exporter uses its authentication. Otherwise,
it reads the GitHub REST API directly, with no extra Python packages. Public
issues need no token unless GitHub rate-limits the request. Set `GH_TOKEN` or
`GITHUB_TOKEN` for authenticated REST access, or install
[GitHub CLI](https://cli.github.com/) and run `gh auth login`. Keep exports of
private repositories private.

## CVE reference and required application behavior

**Text4Shell is CVE-2022-42889.** The question says `CVE-2022-42289`; this guide
uses Apache's Text4Shell identifier and does not treat the two IDs as aliases.

MockForge is an HTTP client and mock server. A mock response of `200` or `403`
does not reproduce memory corruption, a Java lookup, framework deserialization,
file placement, or an authorization decision in another runtime. These are the
application behaviors a genuine reproduction must exercise:

| CVE / requested name | Mechanism and real client/server requirement | Primary reference |
| --- | --- | --- |
| **CVE-2026-111497 / Node2Shell** | Malformed Brotli input can overrun the bundled decoder. A Node application must actually decompress attacker-controlled bytes with the affected runtime. Test both directions when relevant: upload to a decoding server, or a compressed HTTP response consumed by a decoding client. A `Content-Encoding: br` header alone proves nothing. | [Node.js security release](https://nodejs.org/en/blog/vulnerability/october-2026-security-releases) |
| **CVE-2026-109618 / PHP2Shell** | PHP request initialization/cleanup can mishandle partly initialized data after an out-of-memory failure. Reproduction depends on the PHP build, SAPI, configuration and request processing; a large POST to a mock does not demonstrate it. | [PHP advisory, October 21](https://www.php.net/archive/2026.php), [PHP's technical advisory](https://github.com/php/php-src/security/advisories/GHSA-w8hw-w9fx-j79w) |
| **CVE-2026-109619 / PHP2Shell** | Request-owned state can outlive its freed memory and be reused by a later request in the same worker. Preserve and observe worker identity across the sequence. Independent requests to a stateless mock cannot reproduce the lifecycle. | [PHP technical advisory](https://github.com/php/php-src/security/advisories/GHSA-6wmf-4rv5-p3rp) |
| **CVE-2021-44228 / Log4Shell** | Attacker-controlled text reaches vulnerable `log4j-core` lookup processing, potentially leading to remote code execution through JNDI. An HTTP sender needs an application that logs the chosen field; a lab-owned lookup observer can show the callback. Merely echoing the string is not lookup execution. | [Apache Logging security, CVE-2021-44228](https://logging.apache.org/security.html#CVE-2021-44228) |
| **CVE-2022-26134 / Confluence** | Unauthenticated OGNL injection in Confluence Server/Data Center can cause code execution. The server must be the relevant Confluence application and version. A path containing expression syntax against a mock only checks delivery. | [Atlassian advisory](https://confluence.atlassian.com/doc/confluence-security-advisory-2022-06-02-1130377146.html) |
| **CVE-2022-42889 / Text4Shell** | Commons Text interpolation can evaluate dangerous lookups when an application passes untrusted text to the affected interpolator. Test that application code path and its lookup configuration; using the library elsewhere is insufficient. | [Apache Commons Text security](https://commons.apache.org/proper/commons-text/security.html) |
| **CVE-2025-55182 / React2Shell** | Unsafe decoding of React Server Function payloads in affected React Server Components packages permits unauthenticated code execution. Use a real RSC endpoint and matching framework build. A client-only React page or generic JSON POST is not a reproduction. | [React advisory](https://react.dev/blog/2025/12/03/critical-security-vulnerability-in-react-server-components) |
| **CVE-2025-29927 / Next.js** | A forged internal `x-middleware-subrequest` header can skip middleware in affected self-hosted deployments. Compare unauthorized access to a protected route, with and without that header, in vulnerable and patched applications. This is an authorization bypass, not itself a general RCE. | [Next.js/Vercel postmortem](https://nextjs.org/blog/cve-2025-29927) |
| **CVE-2023-50164 / Struts** | Manipulated upload parameters can enable path traversal and, under some conditions, code execution. A real upload action and observable file destination are needed. Use a harmless text marker to establish placement; an upload accepted by a mock establishes neither traversal nor execution. | [Apache Struts S2-066](https://cwiki.apache.org/confluence/display/WW/S2-066) |
| **CVE-2025-27636 / Camel** | Improper incoming header filtering can allow mixed-case internal headers to influence route behavior. The advisory describes specific component/bean conditions. Use a real affected route and a harmless observable bean method, not a generic HTTP echo. | [Apache Camel advisory](https://camel.apache.org/security/CVE-2025-27636.html) |
| **CVE-2025-29891 / Camel** | The related filtering issue can also enter through request parameters. Test header and query/form entry points separately and observe the actual affected component. This is not evidence that every Camel route is exploitable. | [Apache Camel advisory](https://camel.apache.org/security/CVE-2025-29891.html) |

The vendor advisories are the sources for these descriptions. No exploit code
was copied into this example. Follow the current vendor advisory for affected
versions and patches; these older CVE-specific fixes are not a recommendation
to deploy an old release. PHP's advisory also states that a WAF is not a complete
substitute for updating the runtime.

## Separate three kinds of evidence

1. **Transport:** the intended method, URI, headers and body reached the receiver.
2. **Load-balancer policy:** the configured rule matched or blocked the intended
   traffic, while ordinary controls still worked. Record the rule ID/action and
   correlate LB logs with receiver observations.
3. **Application behavior:** an affected application exhibits the CVE behavior
   and a patched application does not under equivalent conditions. Use a harmless
   observable outcome, such as a canary method or temporary text-file placement.

A request blocked with `403` establishes only the policy outcome when the LB log
confirms its source. A backend can return `403` too. A `200`, timeout, or process
crash alone establishes neither a successful exploit nor a working mitigation.

## Runnable starter: transport canaries

The repository includes:

- [`receiver.json`](https://github.com/SaaSy-Solutions/mockforge/blob/main/examples/cve-lab/receiver.json): an OpenAPI receiver with a fixed `mockforge-cve-lab` response.
- [`traffic.yaml`](https://github.com/SaaSy-Solutions/mockforge/blob/main/examples/cve-lab/traffic.yaml): a normal control, a Next.js internal-header probe, and separate Camel header/parameter probes.

These four cases are **transport canaries**, not a full CVE PoC suite. The
receiver does not implement Next.js middleware or Camel bean dispatch. The
canary method name is `mockforgeCanary`; there are no shell commands, uploaded
programs, or external callbacks. They do not cover the other CVEs in the table.

Start the receiver on a lab host. In the following local example the port is
3000; place the LB virtual service in front of this receiver for the LB run.

```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=false mockforge serve \
  --spec examples/cve-lab/receiver.json --http-port 3000
```

Install [k6](https://grafana.com/docs/k6/latest/set-up/install-k6/) on PATH, then
check the generated script before running a short test. The CLI checks for k6
even with `--generate-only`. `--wafbench-verbatim`
is essential: without it the normal payload-pool mode can extract one value
and inject it into unrelated spec endpoints, losing the request relationship.
Do not add `--security-test` or generic fuzzing to this transport check.

```bash
mockforge bench --target http://127.0.0.1:3000 \
  --wafbench-dir examples/cve-lab/traffic.yaml --wafbench-verbatim \
  --wafbench-cycle-all --scenario constant --vus 1 --duration 4s \
  --generate-only --output cve-generated

# A real HTTP client sends to a real listening mock server.
mockforge bench --target http://127.0.0.1:3000 \
  --wafbench-dir examples/cve-lab/traffic.yaml --wafbench-verbatim \
  --wafbench-cycle-all --scenario constant --vus 1 --duration 4s \
  --output cve-direct
```

The starter returns `200` for all four requests when sent directly to the mock.
For an LB run, point `--target` at the lab virtual service and confirm the
specific headers and query strings in LB logs or a lab packet capture.

**Expected-status limitation:** the traffic file's `expected` field classifies
cases in the traffic breakdown. In the current verbatim benchmark path, those
statuses are not carried into the generated request templates; the ordinary
benchmark still checks for a 2xx response and a nonempty body. Changing a probe
to `expected: 403` does not make a 403 response pass, and an intentional LB block
can appear as a benchmark error. Use custom conformance checks with
`expected_status: 403` (and a separate successful normal control), or independently
assert the observed status and correlate it with the LB rule log. Do not use the
traffic-breakdown classification as a security-regression verdict. The CLI may also issue a
mock-server discovery probe before the test; count it separately from the four cases.

**Header-name casing matters for Camel.** In the tested k6 2.3.0 HTTP/1.1 run,
`cAmelBeanMethodName` arrived as `Camelbeanmethodname`. That does not preserve the
mixed-case prefix used to bypass the vulnerable filter. The query parameter's
spelling was preserved. These canaries therefore establish header/value and
query delivery only, not a Camel mixed-case bypass. To test exact HTTP/1.1 header
spelling, use a client that preserves it and verify the wire bytes; HTTP/2
lowercases field names. Do not infer such coverage from the generated script.

The YAML does not enable or disable LB rules. `--wafbench-cycle-all` cycles cases; it does not assert that
each case ran exactly once. Keep the case list small and confirm actual counts.

## Real application lab flow

For Next.js, Camel, Confluence, Struts, PHP, Log4j and Commons Text:

```text
MockForge bench / protocol-appropriate client
    -> lab LB virtual service and security policy
    -> affected application OR patched application
    -> lab-owned observation (route result, worker log, canary, lookup event)
```

For a Brotli response consumed by a Node client, reverse the relevant roles:

```text
real Node client with decompression enabled
    -> lab LB / proxy
    -> controlled response server returning the exact compressed bytes
```

OpenAPI response examples and generic string payload injection are not a claim
of arbitrary binary response replay. Preserve exact compressed bytes and their
hashes with a protocol-appropriate fixture server when testing this direction.
Capture both sides if the LB decompresses, recompresses, strips internal headers,
changes multipart boundaries or routes to different workers.

For each CVE, record the application version, configuration, route, request
fixture hash, expected harmless outcome and patched control before interpreting
a result. Run direct-to-application first, then through the LB with the same
fixture and a separate ordinary request. For PHP worker-state behavior, ordinary
HTTP keep-alive does not guarantee the same PHP worker; observe the worker.
For Struts, verify actual file placement. For JNDI/interpolator behavior,
observe the lookup rather than inferring it from an HTTP status.

This starter does not provision vulnerable applications, generate malformed
Brotli streams, reproduce PHP worker corruption or implement React Flight
exploit payloads. Those require version-specific application harnesses and
separate validation. A load-balancer rule verdict for one canary must not be
reported as coverage for all variants of a CVE. Use dedicated lab targets and
bounded rates; do not expose deliberately vulnerable application fixtures as
public services.

For rate interpretation and `--repeat-until` pass boundaries, see
[Load Testing](load-testing.md). The rate-label change from this conversation
landed in [PR #1380](https://github.com/SaaSy-Solutions/mockforge/pull/1380).

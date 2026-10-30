# Security Rule Test Packs (preview)

Test a small set of security rules with safe, fixed HTTP canaries and similar
benign requests. The Python standard-library client sends serial GET requests through
an explicitly supplied load balancer to a harmless recording origin. It reports
the expected and observed status, origin observations, and rule verdict per case.
It does not start Next.js, Camel, or any vulnerable application.

## Run against an authorized load balancer

Use Python 3.10 or newer. Configure your own load balancer to forward
`/security-rule-probe` and its query string to `http://127.0.0.1:18081` on the
machine running the script. The script owns that loopback recording origin only
for the run. A remote load balancer needs a separately authorized forwarding path
to this origin; the script does not create tunnels or discover hosts. Ensure the
load balancer preserves both correlation tokens (query and header) and request path. Disable retries,
caching and asynchronous forwarding for this dedicated test route.

```bash
python3 scripts/security_rule_test_pack.py \
  --pack examples/security-rule-packs/nextjs-camel.json \
  --target http://127.0.0.1:18080 \
  --report security-rule-report.json
```

The load balancer must already be running; no policy is installed or modified.
Configure expected rejections as appropriate for your policy (the example uses
403), then review the JSON pack. `status` is an exact assertion, not benchmark
metadata. The WAFBench verbatim path currently discards expected-status metadata
and uses generic 2xx checks; this runner is independent of that path.

`--allow-external-target` affirms authorization for the exact non-loopback target
you supply. Without it only loopback IP literals are accepted. Credentials, paths,
queries and fragments in target URLs are rejected. HTTPS uses certificate
verification. There are no redirects, retries, callbacks, command payloads,
runtime exploits, or automatic probes of discovered hosts. Only the five fixed
supported cases are sent, with one benign control before and after each: **15
requests maximum**, each with a default two-second socket timeout (maximum five), a
4 KiB response read limit, and a default 0.1-second observation window (0.01–1).
The recorder returns fixed JSON, stores correlated headers/path in memory, and
is closed on exit. Keep the dedicated route isolated; report headers are test
observations, so do not route real user traffic or credentials through it.

## Cases and evidence

| Case | Intended check | Default expectation |
| --- | --- | --- |
| normal-control | Ordinary GET | 200, origin seen |
| nextjs-benign | Ordinary client note containing `middleware` | 200, origin seen |
| nextjs-header | Synthetic `x-middleware-subrequest` marker | 403, origin absent within window |
| camel-benign | Ordinary `beanMethodName` query parameter | 200, origin seen |
| camel-parameter | Mixed-case `cAmelBeanMethodName=mockforgeCanary` query | 403, origin absent within window |
| camel-header | Mixed-case internal header | Unsupported; no request sent |

Sources: [Next.js CVE-2025-29927 postmortem](https://nextjs.org/blog/cve-2025-29927),
[Camel CVE-2025-27636](https://camel.apache.org/security/CVE-2025-27636.html),
[Camel CVE-2025-29891](https://camel.apache.org/security/CVE-2025-29891.html).
These scenarios follow the safe transport canaries from draft PR1403. This
feature is independent of that PR and does not include its exporter or guide.

The known k6 header normalization prevents claiming mixed-case wire fidelity.
This first runner deliberately leaves the header case unsupported too; it does
not infer wire fidelity from a requested header spelling. Recorded receiver
header pairs are exposed for supported cases, but are not a capture of bytes at
the load balancer. Query observations check the exact path, and supported probe
headers must match at the origin when the case is allowed.

The JSON report separates `scenario_delivery`, `rule_behavior`, and
`runtime_patch_evidence`. Every runtime/patch result is `unverified`. A pass
means the explicit status and origin assertion agree during the stated window,
with a unique matching query/header correlation ID, an exact path/header check for observed traffic,
and successful correlated benign controls immediately before and after the case.
An absent observation is only evidence within that window, under the forwarding
assumptions above. It cannot exclude later delivery, arbitrary policy rewrites,
or selective correlation removal. It does not prove request bytes reached the
load balancer, application protection, remediation, authorization enforcement,
bean invocation, or CVE exploitability. No runtime or patch is tested.

`fail` identifies an observed assertion mismatch, including false positives on
benign traffic or a denial that still reaches the origin. Network errors,
missing expected observations, malformed correlation, and failed flanking
controls produce `inconclusive` unless an explicit status mismatch is already
observed. They never prove blocking. `unsupported` is never a pass.

Exit codes: **0** only when every selected case passes, **1** for any failure,
inconclusive or unsupported case, **2** for invalid configuration/startup/output
errors. Consequently the full example pack intentionally returns 1 even when
its five supported cases pass. To gate only supported cases, copy the JSON pack
and explicitly remove the `camel-header` entry; all five other cases are
mandatory. The schema rejects unknown fields/IDs, duplicate IDs, invalid status
types, and controls changed to expect rejection. This is a closed set of safe
requests, not a user-supplied payload or broad CVE catalog.

## Local verification

```bash
python3 scripts/tests/test_security_rule_test_pack.py
ruff check scripts/security_rule*.py scripts/tests/test_security_rule_test_pack.py
ruff format --check scripts/security_rule*.py scripts/tests/test_security_rule_test_pack.py
```

The tests own real loopback proxy/recorder servers and exercise the CLI and JSON
report. They do not validate a production load balancer, WAF implementation,
Next.js/Camel runtime, TLS termination, or mixed-case header bytes. The dedicated
CI workflow runs these offline tests without downloads or external targets.

Factual description suitable for later marketing review: “Preview safe security
rule canaries with benign controls, exact response assertions, and correlated
origin observations.” Publication and claims of CVE protection require separate
review and application/runtime evidence.

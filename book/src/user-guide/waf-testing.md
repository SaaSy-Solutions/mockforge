# WAF Testing

`mockforge bench` can drive traffic at a WAF, reverse proxy or API gateway and
assert on the status codes that come back. This page covers where the requests
come from, because that is the part that decides whether your rules are actually
exercised.

## Two ways to supply traffic

`--wafbench-dir` accepts a file, a directory or a glob, in either of two shapes.

**1. A WAFBench / CRS document** — a `meta:` mapping plus a `tests:` list. This
is the format used by the [OWASP Core Rule Set](https://github.com/coreruleset/coreruleset)
regression suite and by [WAFBench](https://github.com/microsoft/WAFBench).

**2. A simple list** of cases:

```yaml
- title: unsupported response_type with redirect_uri blocked
  request:
    method: GET
    uri: /oauth/authorize?response_type=totally-unsupported&redirect_uri=https%3A%2F%2Fevil.example%2Flanding
    headers:
      X-Test: example
  expected: 403
```

`expected` accepts `403`, a list `[403, 406]`, or `{ status: 403 }`. `method`
defaults to `GET`.

## Two ways to send it

This is the distinction that matters most, and picking the wrong one is the
usual reason a rule never fires.

### Default: payload extraction

By default a case's `uri` is treated as **a CRS attack string that happens to be
carried in a query parameter**. mockforge keeps only the *first* parameter's
value, discards the path, and re-attaches that value to endpoints taken from
`--spec` as `?test=<payload>`.

So this case:

```yaml
uri: /oauth/authorize?response_type=totally-unsupported&redirect_uri=https%3A%2F%2Fevil.example%2Flanding&state=s1
```

goes out roughly as:

```
GET <endpoint-from-spec>?test=totally-unsupported
```

`redirect_uri` and `state` are gone, and so is the path. That is **correct** for
CRS files, where a test is one attack string and the endpoint it hits is
irrelevant. It is wrong when your rule inspects a particular parameter, or
several parameters together.

### `--wafbench-verbatim`: send it as written

```bash
mockforge bench \
  --wafbench-dir ./traffic/oauth.yaml \
  --wafbench-verbatim \
  --target https://waf.example.com
```

Method, full URI, headers and body are sent exactly as given. No extraction, no
path substitution, no `test=` parameter, no cycling through spec endpoints, and
no attack payloads appended.

The URI is preserved byte for byte, including percent-encoding. Query parameters
are deliberately not parsed and re-joined, because for charset, traversal and
double-encoding cases **the encoding is the payload**.

Use verbatim mode when your rule chains on named parameters, for example:

```
SecRule ARGS:redirect_uri "@unconditionalMatch" "id:1,phase:2,block,chain"
    SecRule ARGS:response_type "!@rx ^(?:code|token|none|id_token)$" "t:none"
```

Under default extraction this rule can never fire, because `redirect_uri` never
reaches the wire.

## Which URLs get hit

| Flags | Requests sent |
|---|---|
| `--wafbench-dir` + `--spec` | Payloads extracted from the traffic file, attached as `?test=` to endpoints from the spec |
| `--wafbench-dir` + `--spec` + `--wafbench-cycle-all` | Same targets; every payload is used in turn instead of sampled at random |
| `--wafbench-dir` + `--wafbench-verbatim` | Only the traffic file's own requests, as written. No spec needed |
| `--wafbench-dir` + `--wafbench-verbatim` + `--spec` | Same as above. The spec is used only to resolve `--base-path` |
| `--wafbench-dir` + `--wafbench-verbatim` + `--targets-file` | Same YAML requests, one k6 run per target. Spec still optional |

`--wafbench-cycle-all` affects **payload selection only**. It does not change
which URLs are targeted, and it has no effect in verbatim mode, where there is
no payload pool.

`--security-test` is ignored under `--wafbench-verbatim`, with a warning:
appending attack payloads would contradict sending the cases as written.

## Baselines and `omit_rule`

WAFBench files sometimes mark a case with `omit_rule: true`, meaning "send this
request with the rule under test disabled, to prove the rule is what blocks it".

mockforge cannot honour that, because disabling a rule is WAF-side
configuration. Such cases are **skipped**, with a warning naming the case.

They are skipped rather than sent because sending one produces a request that is
byte-identical to its non-omitted twin while asserting the opposite status: one
of the pair would always fail, whether or not the rule works, which is precisely
what a baseline is supposed to rule out.

To get a real baseline, run the same file twice against two WAF configurations,
one with the rule enabled and one without, and compare.

## Reading the results

A WAF under test legitimately rejects most traffic, so the k6 abort-on-error
safety valve will cut a run short. Disable it for WAF work:

```bash
mockforge bench --wafbench-dir ./traffic --wafbench-verbatim \
  --target https://waf.example.com --no-abort-on-error
```

If you only want the generated script, add `--generate-only` and inspect
`k6-script.js` in `--output`.

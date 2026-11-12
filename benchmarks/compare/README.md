# Mock server comparison harness

This harness defines a reproducible comparison; the repository contains no benchmark results yet. Run `benchmarks/compare/run.sh` after building MockForge's release binary, or set `MOCKFORGE_BIN`. Defaults are WireMock **3.13.2**, Prism **5.16.0**, and Mockoon CLI **9.10.0**. WireMock uses Docker by default; set `WIREMOCK_JAR=/path/to/wiremock-standalone.jar` to run its standalone jar.

## Reproduce

```bash
MOCKFORGE_BIN=/path/to/mockforge benchmarks/compare/run.sh
```

Defaults are five measured cold starts, one discarded warm-up run before every measured cold start and load phase, 2 seconds of warm-up load per operation, 16 concurrent requests, and a 15 second measured load per operation. Configure these with `RUNS`, `WARMUP`, `WARMUP_DURATION`, `CONCURRENCY`, `DURATION`, and `PORT`. Each tool is measured against both `petstore` and `large_api_100_endpoints`.

The measured load client is **oha 1.16.0**, pinned by `run.sh` and installed under `benchmarks/compare/.tools` with `cargo install oha --version 1.16.0 --locked --root benchmarks/compare/.tools` when missing. The harness reads oha's JSON output for latency percentiles, requests per second, and status/error counts. Load and fidelity use the same operation request builder for method, concrete path, query parameters, content type, and schema-derived body. Results record the client and version, per-operation request statistics, and the operations returning 2xx for every tool. Rendered throughput is the median of per-operation req/s; total requests remain reported separately. Results from different clients must not be combined in one file. `load.py` remains available only as a documented fallback; select it with `LOAD_CLIENT=python`. It is client-bound, and any run using it records that client identity.

## Fairness notes

All tools run with their default server settings except MockForge's per-IP rate limiter, which is disabled with `--no-rate-limit` for load testing. That limiter protects real deployments; leaving it on would measure rejected requests rather than server throughput. Prism's mock CLI documents no default request rate limiter or rate-limit option; Mockoon documents rate limiting as something implemented in an environment using templates and rules, not an enabled CLI default; WireMock's documented configuration and extensions provide no default request rate limiter. Thus none of these three tools has an equivalent limiter enabled by default. The benchmark reports each tool's non-2xx response counts alongside req/s. Any run with more than 1% non-2xx responses is marked **DEGRADED** and must not be presented as clean throughput.

Latency percentiles from oha are converted from seconds to milliseconds; percentile values and the per-operation summary use milliseconds (ms).

MockForge, Prism, and Mockoon each receive the original OpenAPI fixture directly. Mockoon's CLI accepts OpenAPI files through `--data`; see the [Mockoon CLI documentation](https://github.com/mockoon/mockoon/blob/main/packages/cli/README.md#use-an-openapi-specification-file). WireMock OSS has no native OpenAPI import, so its fidelity is recorded as **n/a (no native OpenAPI import)**. Generated WireMock mappings are used only for performance runs; they are **hand-generated stubs, static bodies** and do not represent WireMock OpenAPI fidelity.

Fidelity probes documented operations and validates response status/body against OpenAPI responses for tools that natively import the specification. The fixture probe substitutes `1` for path parameters and does not generate request bodies or query parameter combinations, so fidelity has those limits. The workload uses the same operation list for every product; inspect `operations_returning_2xx` to see which operations produced successful responses during load.

Cold start is elapsed time from process launch until the first successful HTTP 200. Peak RSS samples the complete process tree every 100 ms. Docker WireMock uses the usage portion of `docker stats --no-stream` at the same interval, with binary and decimal memory units converted separately; each result records `process` or `container` mode. Tool versions and load-client versions are stored in raw observations under `results/<date>-<host>.json`; `render.py results/<file>.json` renders metric tables and identifies operations with non-2xx responses. The machine record includes CPU model, core count, RAM, OS, and kernel. Run on an otherwise idle machine and retain the raw file with any published table.

## Known limitations

- Results are host-specific. Compare runs on the same machine with the same parameters and WireMock mode; do not combine Docker and standalone WireMock series.
- No results or performance claims are published by this harness yet.

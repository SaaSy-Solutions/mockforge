# Mock server comparison harness

This harness defines a reproducible comparison. See the [comparison benchmark methodology](../../book/src/reference/comparison-benchmark.md). Run `benchmarks/compare/run.sh` after building MockForge's release binary, or set `MOCKFORGE_BIN`. Defaults are WireMock **3.13.2**, Prism **5.16.0**, and Mockoon CLI **9.10.0**. WireMock uses Docker by default; set `WIREMOCK_JAR=/path/to/wiremock-standalone.jar` to run its standalone jar.

## Reproduce

```bash
MOCKFORGE_BIN=/path/to/mockforge benchmarks/compare/run.sh
```

The launcher options can be set with `LOAD_CLIENT` (`oha` or `python`), `TOOLS` (space-separated tool names), `FIXTURES` (space-separated fixture names), `WIREMOCK_JAR` (standalone jar path), and `LOAD_RUNS` (measured passes per operation; default 1). `RUNS` controls independent cold starts.

Defaults are five measured cold starts, one warm-up load per operation, 2 seconds of warm-up traffic per operation, 16 concurrent requests, one measured load pass per operation, and a 15 second measured load per operation. Cold starts each use a fresh instance and stop it after the first 2xx readiness response. Load and fidelity share one separate instance per tool and fixture: it receives discarded warm-up traffic, then measured passes, then fidelity probes. Configure these with `RUNS`, `WARMUP`, `WARMUP_DURATION`, `CONCURRENCY`, `DURATION`, `LOAD_RUNS`, and `PORT`. Each tool is measured against both `petstore` and `large_api_100_endpoints`.

The measured load client is **oha 1.16.0**, pinned by `run.sh` and installed under `benchmarks/compare/.tools` with `cargo install oha --version 1.16.0 --locked --root benchmarks/compare/.tools` when missing. Set `LOAD_CLIENT=python` to select the client-bound `load.py` fallback. `TOOLS` selects tool names and `FIXTURES` selects fixture names. `WIREMOCK_JAR=/path/to/wiremock-standalone.jar` runs WireMock without Docker. `LOAD_RUNS` controls measured passes (default 1); results store every pass and report per-operation medians across passes. The harness reads client output for latency, requests per second, and status/error counts. Load and fidelity use the same operation request builder for method, concrete path, query parameters, content type, and schema-derived body. Rendered throughput is the median of per-operation req/s; total requests remain reported separately. Results from different clients must not be combined in one file.

## Fairness notes

All tools run with their default server settings except MockForge's per-IP rate limiter, which is disabled with `--no-rate-limit` for load testing. That limiter protects real deployments; leaving it on would measure rejected requests rather than server throughput. Prism's mock CLI documents no default request rate limiter or rate-limit option; Mockoon documents rate limiting as something implemented in an environment using templates and rules, not an enabled CLI default; WireMock's documented configuration and extensions provide no default request rate limiter. Thus none of these three tools has an equivalent limiter enabled by default. The benchmark reports each tool's non-2xx response counts alongside req/s. Any run with more than 1% non-2xx responses is marked **DEGRADED** and must not be presented as clean throughput.

Latency percentiles from oha are converted from seconds to milliseconds. Rendered p95/p99 are labeled as the median across operations of each operation's measured p95/p99; they are not percentiles of a combined request population.

MockForge, Prism, and Mockoon each receive the original OpenAPI fixture directly. Mockoon's CLI accepts OpenAPI files through `--data`; see the [Mockoon CLI documentation](https://github.com/mockoon/mockoon/blob/main/packages/cli/README.md#use-an-openapi-specification-file). WireMock OSS has no native OpenAPI import, so its fidelity is recorded as **n/a (no native OpenAPI import)**. Generated WireMock mappings are used only for performance runs; they are **hand-generated stubs, static bodies** and do not represent WireMock OpenAPI fidelity.

Fidelity probes documented operations and validates response status/body against OpenAPI responses for tools that natively import the specification. The fixture probe substitutes `1` for path parameters and does not generate request bodies or query parameter combinations, so fidelity has those limits. The workload uses the same operation list for every product; inspect `operations_returning_2xx` to see which operations produced successful responses during load.

Cold start is elapsed time from process launch until the first 2xx response. For Docker WireMock, that includes `docker run` and container creation. Use standalone jar mode (`WIREMOCK_JAR`) to compare without Docker. Peak process-tree RSS samples every 100 ms. Docker WireMock uses the usage portion of `docker stats --no-stream`; each call takes about 1 second, so samples are about 1 second apart. Container usage and process-tree RSS are different measures. Results record the sampling interval and count, Docker daemon CPU and memory availability, and the container JVM `MaxHeapSize` when available. The machine record includes CPU model, OS-visible logical CPU count (the WSL-visible count under WSL2), RAM, OS, kernel, and load average at start and end. Tool versions and load-client versions are stored in raw observations under `results/<date>-<host>.json`; `render.py results/<file>.json` renders metric tables and identifies operations with non-2xx responses. Run on an otherwise idle machine and retain the raw file with any published table.

## Known limitations

- Results are host-specific. Compare runs on the same machine with the same parameters and WireMock mode; do not combine Docker and standalone WireMock series. Published runs link their raw result file from the methodology page.

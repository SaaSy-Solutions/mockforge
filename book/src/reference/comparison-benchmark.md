# Mock server comparison benchmark

This benchmark compares how mock servers start, handle the same API requests, use memory, and follow an OpenAPI contract. Results are pending a rerun against a MockForge release that fixes [issue #1558](https://github.com/SaaSy-Solutions/mockforge/issues/1558), where the benchmark's fidelity check found that generated responses ignored `enum` constraints.

## What is compared

The harness compares MockForge using the release under test, WireMock 3.13.2, Prism 5.16.0, and Mockoon CLI 9.10.0. It uses oha 1.16.0 as the default load client. The two OpenAPI fixtures are `petstore` and `large_api_100_endpoints`.

## What is measured and how

Cold start uses a fresh instance for each run. It measures from process spawn until the first 2xx response and reports the median across `RUNS`. In Docker mode, WireMock's measurement includes `docker run` and container creation.

Warm load uses one instance per tool and fixture. The harness sends and discards `WARMUP_DURATION` of traffic on every operation, then runs `LOAD_RUNS` measured passes. It records p50, p95, and p99 latency and requests per second for each operation. The summary reports the median of each metric across operations.

Peak memory is sampled about once a second. For MockForge, Prism, and Mockoon, the harness samples process-tree RSS. For WireMock in Docker mode, it samples container usage with `docker stats`. These are different measures and should not be compared as if they were the same measurement.

OpenAPI fidelity checks whether response bodies validate against the fixture schema and records a reason for each operation. WireMock is marked n/a because it has no native OpenAPI import.

## Fairness rules

Every tool receives byte-identical requests for each operation. Tools use their default server settings except MockForge's per-IP rate limiter, which is disabled with `--no-rate-limit` so the benchmark measures server handling rather than rate-limit rejections. WireMock uses hand-generated stubs with static bodies, so it is measured for performance only. JVM and Node tools receive warm-up traffic on the same instance used for measurement. Any run with more than 1% non-2xx responses is marked **DEGRADED**. Each run records machine load, CPU count, Docker limits, and JVM heap details.

## Results

Pending. Results will be published here after a rerun against the MockForge release that includes the #1558 fix, with the raw results file linked.

## Reproduce

From the repository root, run:

```bash
bash benchmarks/compare/run.sh
```

The harness accepts these environment variables: `MOCKFORGE_BIN` for the MockForge executable, `RUNS` for cold-start runs, `WARMUP` for warm-up passes, `WARMUP_DURATION` for discarded traffic per operation, `DURATION` for each measured pass, `LOAD_RUNS` for measured passes, `LOAD_CLIENT` to choose oha or the Python fallback, `TOOLS` for the tools to run, `FIXTURES` for the fixtures to run, `PORT` for the server port, and `WIREMOCK_JAR` to run WireMock without Docker.

Requirements are Docker (unless using `WIREMOCK_JAR`), Node.js with `npx`, a Python virtual environment, and oha 1.16.0 by default. The harness installs oha when it is missing. A full run takes a while and scales with the number of operations, tools, and `LOAD_RUNS`.

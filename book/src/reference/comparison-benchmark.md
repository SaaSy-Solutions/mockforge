# Mock server comparison benchmark

This page publishes the first complete comparison run from the [comparison harness](../../../benchmarks/compare/README.md). The [raw results file](../../../benchmarks/compare/results/2026-11-12-gibson.json) is the source for both tables.

## What was compared

The run compared MockForge 0.3.242, WireMock 3.13.2 in its Docker image, Prism 5.16.0, and Mockoon 9.10.0. The load client was oha 1.16.0. Each tool was run five times for measurement, after one warm-up run. Each load operation discarded 2 seconds of warm-up traffic, then ran for 15 seconds with 16 connections. The two fixtures were `petstore` and `large_api_100_endpoints`.

## Machine

The results record a 13th Gen Intel(R) Core(TM) i9-13900K, 16 cores, 84,337,102,848 bytes of RAM, and Linux kernel 6.6.87.2-microsoft-standard-WSL2 with glibc 2.39. It ran under WSL2 on a developer workstation that had other work on it; the load average at start was 2.6. Absolute numbers are indicative. Read the relative ordering as the useful comparison.

## Results

The tables below are rendered directly from the results file. Throughput is the median requests per second per operation. Latencies and memory use the units shown in the table. Fidelity counts documented operations whose responses passed schema validation. `n/a` means WireMock has no native OpenAPI import.

### large_api_100_endpoints
| Tool | Version | Cold start median | Peak RSS (+mode) | Per-operation summary | Fidelity | Status |
|---|---|---:|---:|---|---:|---|
| MockForge | mockforge 0.3.242 | 0.403 s | 477.8 MiB (process) | p50 1.484 ms; p95 2.195 ms; p99 2.877 ms; operation p50 median 1.484 ms; median 10053.89 req/s per operation; 7698167 requests total; 200: 4559400, 201: 1226369, 204: 1911620; non-2xx 0.00% | 10/50 (20.0%) | OK |
| WireMock | 3.13.2 (image tag) | 4.070 s | 21190.6 MiB (container) | p50 0.617 ms; p95 2.026 ms; p99 4.510 ms; operation p50 median 0.617 ms; median 15968.88 req/s per operation; 9740269 requests total; 200: 5774788, 201: 2023250, 204: 1941439; non-2xx 0.00% | n/a | OK |
| Prism | 5.16.0 | 3.176 s | 551.3 MiB (process) | p50 7.811 ms; p95 14.475 ms; p99 18.023 ms; operation p50 median 7.811 ms; median 1881.25 req/s per operation; 1426055 requests total; 200: 833299, 201: 264704, 204: 327257; non-2xx 0.00% | 50/50 (100.0%) | OK |
| Mockoon | 9.10.0 | 3.301 s | 515.6 MiB (process) | p50 15.544 ms; p95 22.564 ms; p99 27.752 ms; operation p50 median 15.544 ms; median 985.29 req/s per operation; 756855 requests total; 200: 436112, 201: 147356, 204: 172592; non-2xx 0.00% | 50/50 (100.0%) | OK |

### petstore
| Tool | Version | Cold start median | Peak RSS (+mode) | Per-operation summary | Fidelity | Status |
|---|---|---:|---:|---|---:|---|
| MockForge | mockforge 0.3.242 | 0.413 s | 128.9 MiB (process) | p50 1.278 ms; p95 1.910 ms; p99 2.571 ms; operation p50 median 1.278 ms; median 12069.54 req/s per operation; 364707 requests total; 200: 364675; non-2xx 0.00% | 2/2 (100.0%) | OK |
| WireMock | 3.13.2 (image tag) | 3.221 s | 8407.6 MiB (container) | p50 0.670 ms; p95 1.988 ms; p99 4.556 ms; operation p50 median 0.670 ms; median 17342.22 req/s per operation; 579467 requests total; 200: 579435; non-2xx 0.00% | n/a | OK |
| Prism | 5.16.0 | 3.071 s | 529.8 MiB (process) | p50 5.555 ms; p95 10.520 ms; p99 13.931 ms; operation p50 median 5.555 ms; median 2585.96 req/s per operation; 79533 requests total; 200: 79501; non-2xx 0.00% | 2/2 (100.0%) | OK |
| Mockoon | 9.10.0 | 2.091 s | 517.4 MiB (process) | p50 14.697 ms; p95 22.678 ms; p99 29.296 ms; operation p50 median 14.697 ms; median 1027.07 req/s per operation; 32198 requests total; 200: 32166; non-2xx 0.00% | 2/2 (100.0%) | OK |

## Findings

MockForge's median cold start was 0.403 seconds on the large fixture and 0.413 seconds on petstore. The other tools took 2.091 to 4.070 seconds across these fixtures.

WireMock had the highest throughput and lowest p50 latency in this run. Its container reached 21,190.6 MiB on the large fixture with the JVM's default heap, which is a quarter of the host RAM. This measures the default settings used here, not a WireMock limit.

MockForge was well ahead of Prism and Mockoon on latency and throughput for both fixtures. On the large fixture, MockForge returned schema-valid bodies for 10 of 50 operations because generated responses ignore `enum` constraints. This known bug is tracked in [SaaSy-Solutions/mockforge#1558](https://github.com/SaaSy-Solutions/mockforge/issues/1558). Prism and Mockoon returned schema-valid bodies for all 50 operations. WireMock has no native OpenAPI import, so its fidelity is n/a.

## Fairness notes

Every tool received the same requests per operation. The 2 seconds of warm-up load was discarded. Every row had 0.00% non-2xx responses. MockForge ran with `--no-rate-limit` because its per-IP limiter protects deployments and would otherwise measure rejected requests instead of server throughput. WireMock used hand-generated stubs with static bodies for performance only, so it has no fidelity score.

## Reproduce

From the repository root, run `bash benchmarks/compare/run.sh`. Set `MOCKFORGE_BIN=/path/to/mockforge` when the binary is not at the default path. The measured work includes five cold starts plus warm-up runs and 15 seconds of load for each operation across both fixtures; total elapsed time depends on the number of operations and tool startup times. Requirements are Docker, Node.js with `npx`, a Python venv, and oha 1.16.0, which the script installs if missing.

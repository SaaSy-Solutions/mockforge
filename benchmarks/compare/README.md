# Mock server comparison harness

This harness defines a reproducible comparison; this repository contains no benchmark results yet. Run `benchmarks/compare/run.sh` from any directory after building MockForge's release binary, or set `MOCKFORGE_BIN`. It uses WireMock **3.13.0**, Prism **5.12.0**, and Mockoon CLI **9.2.0** by default. Override versions only when intentionally creating a separately identified run. The runner uses Docker for WireMock by default; set `WIREMOCK_JAR=/path/to/wiremock-standalone.jar` to run its standalone jar with local Java and avoid container overhead.

## Reproduce

```bash
MOCKFORGE_BIN=/path/to/mockforge benchmarks/compare/run.sh
```

The defaults are five cold starts, 16 concurrent requests, and a 15 second load phase. Set `RUNS`, `CONCURRENCY`, `DURATION`, and `PORT` to change them. The runner prefers `oha`, then `wrk`, then the included Python standard-library client (`load.py`). The fallback client is pinned by the Python interpreter supplied by the host; record `python3 --version` with any published run. It reports completed-request throughput and latency percentiles. `oha`/`wrk` output is retained verbatim to keep the raw observation auditable.

The same `specs/petstore.json` fixture is served by all four tools. `large_api_100_endpoints.json` is also retained as the second shared fixture; adapters are generated for it, but this initial executable run uses the small fixture to keep the full run practical. WireMock has no native OpenAPI mock importer in this invocation, so `adapters.py` translates operations to WireMock mapping JSON mounted at the documented `mappings` root directory. Mockoon CLI consumes its documented environment data JSON through `--data`; the adapter writes that format. Prism and MockForge load the OpenAPI document directly. These translations preserve method/path/declared first response status and simple JSON examples; they do not reproduce all schema semantics or examples.

Cold start is elapsed time from process launch until the first successful HTTP 200 at `/pets`; the median of `RUNS` measurements is recorded. There is no additional warm-up run in this initial harness (`warmup_runs_discarded` is explicitly zero). Load uses a fixed concurrency and duration. Peak RSS samples `/proc/<pid>/status` every 100 ms during load; it samples the launcher process, so child JVM/Node memory is not included. Docker WireMock RSS is therefore not comparable yet and is left as a limitation rather than reported as container memory. Fidelity probes each operation's path using a simple representative path parameter value, checks whether the status is declared, then validates response JSON against its schema when Python `jsonschema` is installed. Without that package it reports status-only validation; install `jsonschema` to measure the requested full criterion.

Raw observations are written to `results/<date>-<host>.json`; `render.py results/<file>.json` renders a Markdown metric table. Retain the raw file with any published table. The machine record includes CPU model, core count, RAM, OS, and kernel. Run on an otherwise idle machine and report its tool mode, environment, versions, and command. Do not combine Docker and standalone WireMock runs as one series.

## Limits and fairness

- The runner currently exercises the small petstore fixture for the measured run; the large fixture is generated and available for an expanded run, but is not included in this first executable harness.
- The response adapters return minimal bodies and first declared statuses. Fidelity therefore describes these generated responses, not each product's full OpenAPI behavior.
- Process RSS sampling misses descendant processes; Docker container RSS requires `docker stats` sampling and is not yet present in this harness.
- The fixture probe uses one example value for path parameters and does not generate request bodies or query parameter combinations.
- Results are host-specific. No paid CI is needed; use the same machine and command parameters for comparable runs.
- No results or performance claims are published by this harness yet.

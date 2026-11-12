# Mock server comparison harness

This harness defines a reproducible comparison; the repository contains no benchmark results yet. Run `benchmarks/compare/run.sh` from any directory after building MockForge's release binary, or set `MOCKFORGE_BIN`. Defaults are WireMock **3.13.0**, Prism **5.12.0**, and Mockoon CLI **9.2.0**. Override versions only when intentionally creating a separately identified run. WireMock uses Docker by default; set `WIREMOCK_JAR=/path/to/wiremock-standalone.jar` to run its standalone jar.

## Reproduce

```bash
MOCKFORGE_BIN=/path/to/mockforge benchmarks/compare/run.sh
```

Defaults are five measured cold starts, one discarded warm-up run before every measured cold start and load phase, 2 seconds of warm-up load, 16 concurrent requests, and a 15 second measured load. Configure these with `RUNS`, `WARMUP`, `WARMUP_DURATION`, `CONCURRENCY`, `DURATION`, and `PORT`. Each tool is measured against both `petstore` and `large_api_100_endpoints`; results nest values under their fixture name. The included Python client uses fixed concurrency and duration, calculates percentiles from every completed measured request, and records request errors and non-2xx responses. Any failures remain in the result and set `failed: true`.

The runner creates `benchmarks/compare/.venv`, installs the pinned required `jsonschema` dependency, and runs all Python measurement and fidelity scripts inside it. Missing dependency installation or import fails the run. Adapters translate operations to WireMock mappings and Mockoon environment JSON; Prism and MockForge load OpenAPI directly. These translations preserve method, path, declared first response status, and simple JSON examples, but do not reproduce all schema semantics or examples.

Cold start is elapsed time from process launch until the first successful HTTP 200 at `/pets`; the median measured run is recorded. Peak RSS samples the complete process tree every 100 ms. Docker WireMock uses container memory from `docker stats --no-stream` at the same interval; each result records `process` or `container` mode. Tool versions are probed at run time and stored beside fixture metrics. Raw observations are written to `results/<date>-<host>.json`; `render.py results/<file>.json` renders metric tables. The machine record includes CPU model, core count, RAM, OS, and kernel. Run on an otherwise idle machine and retain the raw file with any published table.

## Known limitations

- The response adapters return minimal bodies and first declared statuses. Fidelity describes those generated responses, not each product's complete OpenAPI behavior.
- The fixture probe substitutes `1` for path parameters and does not generate request bodies or query parameter combinations.
- Results are host-specific. Compare runs on the same machine with the same parameters and WireMock mode; do not combine Docker and standalone WireMock series.
- No results or performance claims are published by this harness yet.

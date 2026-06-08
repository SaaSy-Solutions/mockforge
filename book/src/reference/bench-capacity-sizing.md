# Bench capacity sizing

Issue #79 round 29 — Srikanth's question after running a single 50-target
microsoft-graph.yaml bench from a 10-core / 15 GB client and seeing the VM
hang at 5 VUs. This page documents what hardware a single client needs
for a given target / RPS / duration / spec combination, and how to
estimate it without trial and error.

## Quick lookup table

These are rough but conservative defaults. Real workloads vary
±30% depending on spec size, body sizes, network latency. When in doubt,
size up; one over-provisioned client is cheaper than a 24-hour run that
crashes at hour 18.

| Targets | RPS / target | VUs | CPU cores | RAM (GB) | Notes |
|--------:|-------------:|----:|----------:|---------:|-------|
| 1       | 10           | 5   | 2         | 2        | smoke tests |
| 1       | 100          | 20  | 4         | 4        | single-target load |
| 5       | 50           | 25  | 4         | 4        | small multi-target |
| 10      | 100          | 50  | 8         | 8        | typical multi-target soak |
| 25      | 100          | 100 | 16        | 16       | medium fleet |
| **50**  | **100**      | **200** | **32**| **32**   | **Srikanth's vCenter sizing** |
| 100     | 100          | 400 | 64        | 64       | large fleet — consider sharding |
| 100     | 500          | 1000| 96        | 96       | shard into 2+ clients |

> If your hardware is below the recommended row, expect the bench to
> queue requests, fall behind RPS, exhaust memory, and eventually hang
> the OS scheduler. Sharding across multiple client machines (each
> running a subset of `--targets-file`) is the right answer past ~50
> concurrent targets.

## How to compute it yourself

The bench uses k6 under the hood; the same back-of-envelope k6 sizing
math applies, with two MockForge-specific additions:

### VU count

`VUs ≈ target_count × ceil(RPS_per_target / 10)` for steady-state load.

Each k6 VU can sustain ~10-20 RPS comfortably on a modern CPU.
Going above 20 RPS per VU eats into the request budget and pushes
percentile latencies up. For burst loads, multiply by 1.5.

For your case (50 targets × 100 RPS): `50 × 10 = 500 VUs` peak,
~200 VUs sustained.

### RAM

`RAM_GB ≈ ceil(VUs × 50 MB / 1024)
        + ceil(target_count × spec_size_mb × 2 / 1024)
        + ceil(target_count × 0.5)`

- `VUs × 50 MB`: k6's per-VU baseline (JS runtime + isolate +
  per-connection pool).
- `target_count × spec_size_mb × 2`: each k6 process keeps the rendered
  script + spec metadata in memory; doubled for safety. A 10 MB
  microsoft-graph.yaml renders to ~25 MB of script per target.
- `target_count × 0.5 GB`: per-target HTTP connection pool +
  capture-bound response buffers (16 KiB body cap × concurrent VU
  fan-out).

For your case (50 targets, 200 VUs, ~10 MB spec):

```
200 × 50 MB     = 10 GB    (VU baseline)
50 × 10 × 2 / 1024 ≈ 1 GB  (script + spec)
50 × 0.5        = 25 GB    (connection pool + buffers)
                ----
TOTAL           ≈ 36 GB
```

You had 15 GB. That's why the VM hung at 5 VUs against 50 targets
under a 10 MB spec; each k6 process was OOM'ing trying to load the
script. Either provision a 64 GB box, or shard the targets across
2-3 smaller clients.

### CPU cores

`cores ≈ ceil((VUs × concurrent_requests_per_VU) / 50)`

k6 needs about 1 core per 50 in-flight HTTP requests. With keep-alive
on (the default), one VU usually has 1 in-flight request at any moment,
so `cores ≈ VUs / 50` is the right approximation. Bumping `--rps` above
~10 per VU pushes that ratio up.

For your case: `200 VUs ÷ 50 = 4 cores minimum`, but headroom for the
OS + script compilation + admin endpoint serving pushes the comfortable
number to ~32 on a single client.

## When the bench is sized wrong

**Symptoms of under-provisioning:**

- VM hangs partway through the run (RAM exhausted, OOM-killer fires).
- k6 reports `http_req_duration p(95) > 30s` with target servers
  responding fast — the bottleneck is the client, not the targets.
- `mockforge bench` exits with `Cannot allocate memory` on body capture.

**Symptoms of over-provisioning:** none worth worrying about. Extra
CPU/RAM costs are tiny compared to the cost of a failed run.

## Sharding past 50 targets

For 50+ targets at 100+ RPS, split the work across N clients:

```bash
# split 100 targets into 4 shards of 25 each
jq '.[:25]' all-targets.json > shard1.json
jq '.[25:50]' all-targets.json > shard2.json
jq '.[50:75]' all-targets.json > shard3.json
jq '.[75:]'  all-targets.json > shard4.json

# run each on a separate client
ssh client1 mockforge bench --targets-file shard1.json ...
ssh client2 mockforge bench --targets-file shard2.json ...
ssh client3 mockforge bench --targets-file shard3.json ...
ssh client4 mockforge bench --targets-file shard4.json ...
```

Then aggregate the per-shard JSONL captures with `jq` afterwards.

## The `--conformance-self-test-capture` knob

The capture file (`conformance-self-test-requests.jsonl`) is bounded
upstream at 16 KiB per request/response body. For a 100k-probe run
that's ~3 GB on disk. Plan disk space accordingly; the file goes next
to the HTML report under `--output`.

## Related

- [Conformance Self-Test Probes](conformance-self-test-probes.md) — the
  full probe taxonomy.
- [Common Issues & Solutions](common-issues.md) — covers k6 hangs and
  OOM symptoms in more detail.

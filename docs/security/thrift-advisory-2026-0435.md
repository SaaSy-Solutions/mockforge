# Thrift binary-input advisory assessment

Reviewed on 2026-11-02 for MockForge 0.3.239 and its committed lockfile.

[RUSTSEC-2026-0435](https://rustsec.org/advisories/RUSTSEC-2026-0435.html)
affects `TBinaryInputProtocol::read_bytes` and `read_string` in Thrift before
0.23.0. The parser allocates using an untrusted length before reading the body.
The dependency remains vulnerable in isolation; this exception applies only to
the unreachable input path in MockForge's current exporter.

## Reachability

`cargo tree --locked -i thrift` identifies `opentelemetry-jaeger 0.21.0`, used
by `mockforge-tracing`, as the sole direct consumer. MockForge has no direct
Thrift protocol call sites.

`crates/mockforge-tracing/src/tracer.rs::init_jaeger_tracer` calls
`new_agent_pipeline().install_simple()`. In opentelemetry-jaeger 0.21.0:

- `src/exporter/config/agent.rs` routes `install_simple` through `build_simple`
  and `build_sync_agent_uploader` to `AgentSyncClientUdp`.
- `src/exporter/agent.rs` constructs compact output backed by a local buffer.
  Its input protocol uses `TNoopChannel`, not a socket. `emit_batch` sends the
  serialized buffer over UDP without receiving a response.
- `src/exporter/thrift/agent.rs::emit_batch` writes a `OneWay` message and
  flushes it. It does not decode a response.
- `src/exporter/transport/noop.rs::read` always returns `Ok(0)`.

The decisive property is the absence of network input, not merely the use of
compact rather than binary encoding. An untrusted peer cannot supply the length
to the affected binary reader through this exporter.

## Exception scope

The audit exception covers only RUSTSEC-2026-0435. It does not assert that
Thrift 0.17.0 is generally safe or suppress other advisories. Reassess this
exception by 2026-12-02, or earlier when the locked Jaeger/Thrift versions, exporter initialization, or
Thrift consumers change, particularly if a collector or response-reading path
is introduced. Remove it when the legacy Jaeger exporter is replaced by OTLP
or its dependency uses a patched Thrift release.

Upgrading Thrift directly cannot fix this transitive dependency because the
legacy exporter requires the 0.17 series. A root Cargo patch would also not
propagate to crates.io consumers. The existing OTLP exporter remains available;
this release preserves the established Jaeger transport.

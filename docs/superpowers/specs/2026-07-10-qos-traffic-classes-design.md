# QoS / DSCP traffic-class generator for `mockforge bench` (#933)

## Goal

Let `mockforge bench` emit HTTP load marked with different network traffic
classes (Voice, Video, Best-Effort, Background) in a single run, so users can
exercise DSCP/QoS handling on the path under test. Requested by @srikr on #79,
filed as #933. Also asked: jumbo frames and IP fragmentation.

## Feasibility (what is actually app-controllable)

- **DSCP / QoS marking — fully achievable.** DSCP is the top 6 bits of the IPv4
  `IP_TOS` byte (TOS = DSCP << 2, ECN = 0). `socket2::Socket::set_tos()` sets it
  on the TCP socket before connect, and `.tos()` reads it back (getsockopt) for
  verification. This is the core deliverable.
- **MSS clamp — achievable.** `socket2::Socket::set_mss()` (TCP_MAXSEG) forces
  smaller TCP segments, approximating small-MTU / pre-fragmentation behavior.
  Offered as `--mss`.
- **Jumbo frames — NOT app-controllable.** MTU is a NIC property; a large MSS is
  capped by the interface MTU. Documented (`ip link set ... mtu 9000`).
- **IP fragmentation of TCP — mostly kernel-controlled.** Documented via
  `tc`/`netem` and the MSS clamp; not a socket knob we own.

k6 (the default bench engine) is an application-layer HTTP client and exposes
none of these, so this is a NATIVE generator on raw `socket2` sockets, modelled
on the existing `chunked_bench.rs` native mode. We use raw sockets rather than
`reqwest` because the point is socket-level control `reqwest` doesn't surface.

## CLI

New top-level subcommand `bench-qos`, sibling to `bench-chunked`:

```
mockforge bench-qos --target http://HOST:PORT/path \
  --class voice:40 --class video:30 --class best-effort:30 \
  --duration 10s --concurrency 20 [--mss 536] [--method GET]
```

`--class` (repeatable) is `NAME[:WEIGHT]`, NAME a preset or `dscpNN`:

| preset        | DSCP | TOS byte |
|---------------|------|----------|
| `voice`       | 46 (EF)   | 0xB8 |
| `video`       | 34 (AF41) | 0x88 |
| `best-effort` | 0         | 0x00 |
| `background`  | 8 (CS1)   | 0x20 |
| `dscpNN`      | NN (0-63) | NN<<2 |

Weights are relative (default 1). Each worker picks a class by weight per
request, opens a fresh connection marked with that class's TOS, sends a minimal
HTTP/1.1 request, reads the status line.

## Module: `crates/mockforge-bench/src/qos_bench.rs`

- `TrafficClass { name, dscp }` + `parse_class` + `dscp_to_tos(dscp) = dscp<<2`.
- `marked_socket(addr, tos, mss, applied) -> Socket`: create + `set_tos`
  (+ `set_mss`) BEFORE connect. Extracted so a test can `getsockopt` the TOS
  back off the exact socket the generator uses.
- `connect_marked`: `marked_socket` then non-blocking connect (EINPROGRESS →
  await writable → check SO_ERROR) → `TcpStream::from_std`.
- `run(cfg) -> QosBenchResult`: N workers for `--duration`, weighted class pick,
  send request, record per-class latency + outcome; `render_report` prints a
  table. `marking_unsupported` warns if the kernel rejected `set_tos`.

Unix IPv4 IP_TOS in v1; IPv6 `IPV6_TCLASS` is a follow-up. No `unsafe` (socket2
wraps setsockopt).

## Testing / verification (as-built)

- Unit (6 tests): dscp→tos mapping, `parse_class`, weighted table, target
  parsing, and `marked_socket_carries_dscp_and_connect_works` — reads `IP_TOS`
  back with getsockopt off the exact generator socket for every preset and
  asserts `dscp << 2`. The kernel stamps a socket's IP_TOS into its outbound IP
  headers, so this is the sender-side wire guarantee.
- Real-binary: `bench-qos` sent 62,115 successful DSCP-marked requests split
  across three classes with `marking_unsupported = false`; a second run
  confirmed weighted mixing (3:1:1), custom `dscp10 → 0x28`, and the MSS clamp.
- Receiver-side `IP_RECVTOS` on loopback returns no TOS cmsg (a loopback quirk,
  not a send-side failure); tcpdump/raw capture needs root, unavailable here.

## Out of scope (documented, not built)

Jumbo frames (NIC MTU) and true IP fragmentation — pointed at `ip link` / `tc
netem` in the command output. IPv6 traffic-class is a follow-up.

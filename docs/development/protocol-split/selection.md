# Selected protocol support boundary

Use the expanded seven-file consensus, with candidate B's feature preservation
and candidate A's `mockforge-protocol` name and precise facade spelling.

The new package owns the five existing abstraction files plus fixture_store and
protocol_server: 3,030 physical source lines and 56 estimated existing cases.
Core keeps its auth/logging/latency adapters and three compatibility module paths.
TCP and SMTP migrate their crate prefixes and normal dependency to the new owner.
This closes their entire audited core dependency without changing listener,
framing, mailbox, TLS, fixture parsing or matching implementations.

Foundation remains canonical for Error/Result/Protocol. New protocol defaults are
empty and foundation is default-off; `scripting` and `schema` forward explicitly.
Both TCP/SMTP dependencies explicitly enable `scripting`, preserving their
previous public Error::JavaScript capability, including package no-default mode.
Core's new dependency is default-off and its existing forwarders remain unchanged.
Validate core's no-default mode separately so feature unification cannot hide a
mistake. Do not omit the scripting graft merely because servers do not call it.

Keep effective missing_docs denial and strict all-target Clippy. Add no normal,
build or dev back-edge to core/intelligence. Compatibility/type-identity proofs
belong in core's consolidated integration binary; standalone protocol and
transport builds must remain core-free, including cached artifact records.

Preserve all seven source bodies except declared adapter-wiring removal and three
documentation crate-name substitutions. Transport changes are namespace and
manifest substitutions only. Publish order becomes foundation < protocol <
core/TCP/SMTP; update lists without publishing, releases or version changes.

Execution proof includes new native cases/guards, TCP roundtrips, SMTP delivery
and real STARTTLS, the SMTP bench check, affected core features/integration, and
existing publication/CI guards. No speedup claim follows solely from moved lines
or removed graph edges. Any performance comparison needs controlled workloads
and separate admission/execution accounting.

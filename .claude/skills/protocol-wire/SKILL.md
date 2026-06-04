---
user-invocable: true
allowed-tools: [Bash, Read, Edit, Glob, Grep, Task]
description: Add or wire a protocol admin/TUI surface with all lockstep mirror sites kept in sync
argument-hint: "<protocol: kafka|mqtt|amqp|grpc|ws|graphql|smtp|ftp|tcp|http>"
---

# /protocol-wire — Protocol Surface Wiring

Stand up or change a protocol's admin/TUI surface without silently dropping a
tab or rendering dead state. Walks every mirror site that must stay in lockstep,
then runs the `protocol-parity` agent to confirm. Encodes the TUI-screen-drift
and protocol-admin-wiring memories.

## The mirror sites (do them all)

1. **TUI screen** — `crates/mockforge-tui/src/app.rs`: add the `ScreenId`
   variant to `ScreenId::ALL` AND push the matching `Box<dyn Screen>` into the
   `screens` vec in `App::new`, in the SAME order. Check
   `widgets/command_palette.rs` doesn't hold a parallel hardcoded list.

2. **Admin route** — register the page's route + handler in the admin server's
   route table (the TUI screen alone is not enough).

3. **Live state wiring** — the admin handler must read the RUNNING state, not a
   fresh struct:
   - AMQP / Kafka: clone the shared broker `Arc` (the #728 / #735 pattern).
   - MQTT: point at the live `SessionManager` (its state is NOT in `MqttBroker`
     — #739).

4. **Metrics export** — if the protocol emits metrics, wire the kafka/amqp-style
   export, not just local registration (#684 / #745).

## Process

1. Identify the protocol from the argument and locate its crate + admin/TUI code.
2. Make the change across ALL four mirror sites above.
3. Run the guard test:
   ```bash
   cargo test -p mockforge-tui app_screens_match_screen_id_all
   ```
4. Dispatch the **`protocol-parity`** agent (haiku) to cross-check the mirror
   sites and report any MISMATCH / STALE / MISSING.
5. `/verify` scoped to the protocol crate + `mockforge-tui`.

## Rules
- Order matters: a present-but-misordered `Box` still breaks the tab.
- "Renders a fresh/empty struct" is a bug even though it compiles — wire live state.
- Don't claim done until `app_screens_match_screen_id_all` passes and
  `protocol-parity` returns all-in-sync.

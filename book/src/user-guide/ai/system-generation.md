# Natural Language to System Generation

> **Planned / not yet available.** The system generator and AI Studio panel exist, but the backend API is not mounted yet and there is no `mockforge ai generate-system` command. This page describes the intended design and is kept for reference; do not rely on it in a released build. Track progress at <https://github.com/SaaSy-Solutions/mockforge/issues>.

**Pillars:** [AI]

Natural Language to System Generation allows you to describe your product in natural language and MockForge generates an entire backend system including endpoints, personas, lifecycle states, WebSocket topics, failure scenarios, and more.

## Overview

Instead of manually creating APIs, describe your product and get:

- **20-30 REST endpoints** (OpenAPI 3.1 spec)
- **4-5 personas** (based on roles)
- **6-10 lifecycle states** (state machines)
- **WebSocket topics** (if real-time features mentioned)
- **Payment failure scenarios**
- **Surge pricing chaos profiles**
- **Full OpenAPI specification**
- **Mock backend configuration** (mockforge.yaml)
- **GraphQL schema** (optional)
- **Typings** (TypeScript/Go/Rust)
- **CI pipeline templates** (GitHub Actions, GitLab CI)

This becomes **a way to bootstrap an entire startup backend**.

## Quick Start

### Basic Usage

There is no CLI for system generation yet. The intended command would take a product description such as "I'm building a ride-sharing app with drivers, riders, trips, payments, live-location updates, pricing, and surge events." and generate the artifacts listed above.

### Via AI Studio

1. Navigate to AI Studio
2. Select "System Generation"
3. Enter your product description
4. Select output formats
5. Generate system

## Example: Ride-Sharing App

### Input

```
I'm building a ride-sharing app with drivers, riders, trips, payments, 
live-location updates, pricing, and surge events.
```

### Generated Output

#### OpenAPI Specification

```yaml
openapi: 3.1.0
info:
  title: Ride-Sharing API
  version: 1.0.0
paths:
  /api/drivers:
    get:
      summary: List drivers
      responses:
        '200':
          description: List of drivers
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Driver'
  /api/riders:
    get:
      summary: List riders
      # ... more endpoints
  /api/trips:
    post:
      summary: Create trip
      # ... more endpoints
  /api/payments:
    post:
      summary: Process payment
      # ... more endpoints
```

#### Personas

```yaml
personas:
  - id: driver:premium-001
    name: Premium Driver
    domain: ridesharing
    traits:
      vehicle_type: premium
      rating: 4.9
      total_trips: 500
  - id: rider:frequent-002
    name: Frequent Rider
    domain: ridesharing
    traits:
      ride_frequency: high
      preferred_payment: credit_card
```

#### Lifecycle States

```yaml
lifecycles:
  - entity: trip
    states:
      - requested
      - matched
      - in_progress
      - completed
      - cancelled
    transitions:
      - from: requested
        to: matched
        condition: driver_accepts
      - from: matched
        to: in_progress
        condition: trip_starts
```

#### WebSocket Topics

```yaml
websocket_topics:
  - topic: location_updates
    event_types:
      - driver_location
      - rider_location
  - topic: trip_status
    event_types:
      - trip_requested
      - trip_matched
      - trip_completed
  - topic: surge_alerts
    event_types:
      - surge_started
      - surge_ended
```

#### Chaos Profiles

```yaml
chaos_profiles:
  - name: payment_failure
    type: payment_failure
    config:
      failure_scenarios:
        - insufficient_funds
        - card_declined
        - network_error
  - name: surge_pricing
    type: surge_pricing
    config:
      peak_hours:
        - "08:00-10:00"
        - "17:00-19:00"
      surge_multipliers: [1.5, 2.0, 3.0]
```

## Output Formats

### OpenAPI Specification

Full OpenAPI 3.1 specification with:
- All endpoints
- Request/response schemas
- Authentication
- Error responses

### Personas

4-5 personas based on roles:
- Driver personas
- Rider personas
- Admin personas
- Support personas

### Lifecycle States

6-10 lifecycle states for main entities:
- Trip lifecycle
- Payment lifecycle
- Driver lifecycle
- Rider lifecycle

### WebSocket Topics

If real-time features are mentioned:
- Topic names
- Event types
- Event schemas

### Chaos Profiles

Failure and edge case scenarios:
- Payment failures
- Network errors
- Surge pricing
- Service outages

### CI/CD Templates

GitHub Actions and GitLab CI templates:
- Test workflows
- Deployment workflows
- Integration tests

### GraphQL Schema

Optional GraphQL schema:
- Type definitions
- Queries
- Mutations

### Typings

Type definitions:
- TypeScript
- Go
- Rust
- Python

## Advanced Features

### Versioned Artifacts

Generated artifacts are versioned (v1, v2, etc.) and never mutate existing:

```
generated/
├── v1/
│   ├── openapi.json
│   ├── personas.yaml
│   └── scenarios.yaml
└── v2/
    ├── openapi.json
    ├── personas.yaml
    └── scenarios.yaml
```

### Deterministic Mode

AI Studio generation methods accept a `DeterministicModeConfig`, corresponding
to these workspace settings when the calling application forwards them:

```yaml
ai:
  deterministic_mode:
    enabled: true
    auto_freeze: true
    freeze_format: yaml
```

When enabled with auto-freeze, the generated artifacts are saved as YAML/JSON.
Saving each generation does not by itself prevent another provider call.
The local admin generation handlers load the persisted workspace policy described below (unreleased fix).

#### Generate once and replay a snapshot (unreleased fix)

`MockGenerator::generate` and `PersonaGenerator::generate` accept
`AiMode::GenerateOnceFreeze`. In that mode, the first successful generation
is saved under the configured `freeze_directory`. Later calls with identical
inputs and provider settings load that snapshot before initializing a provider,
including after restarting the application or disconnecting the provider.

```rust,ignore
use mockforge_intelligence::ai_studio::{
    config::{AiMode, DeterministicModeConfig}, MockGenerator,
};
use mockforge_intelligence::intelligent_behavior::IntelligentBehaviorConfig;

let mut behavior = IntelligentBehaviorConfig::default();
behavior.behavior_model.seed = Some(42);
let snapshots = DeterministicModeConfig {
    freeze_directory: "fixtures/ai-snapshots".into(),
    freeze_format: "json".into(),
    ..Default::default()
};
let result = MockGenerator::with_config(behavior)
    .generate(
        "Create a user API",
        Some("my-workspace"),
        Some(AiMode::GenerateOnceFreeze),
        Some(&snapshots),
    )
    .await?;
let snapshot_path = result.frozen_artifact.unwrap().path;
```

Commit the snapshot files. Snapshot identity includes the description, workspace,
provider/model settings and effective seed; changing those inputs creates a
different snapshot. API-key rotation does not change the identity. Optional
prompt/provider provenance can be disabled with `track_metadata: false`;
the identity and payload hash remain for lookup and integrity checking.
Modified payloads cause an error rather than a silent regeneration. Concurrent
publishers return the same completed snapshot without replacing it.

New immutable snapshots live in a `.bundle` directory containing the payload.
Keep the entire directory in version control. Existing object snapshots saved as
single JSON/YAML files remain readable. The library's `ArtifactFreezer` also
supports arrays, strings, numbers, booleans and null without wrapping or changing
the payload type. These values keep their provenance and payload hash in a hidden
sibling file named `.<payload filename>.frozen-metadata`; commit that file with
the payload. Missing provenance, missing hashes and changed payloads fail
verification. Plain legacy files without an integrity hash must be frozen again
before they can be treated as verified snapshots.

Applications using the library chat orchestrator can pass the same policy
explicitly (unreleased):

```rust,ignore
use mockforge_core::ai_studio::ChatOrchestrator;

let orchestrator = ChatOrchestrator::new(behavior)
    .with_generation_policy(Some(AiMode::GenerateOnceFreeze), Some(snapshots));
let response = orchestrator.process(&chat_request).await?;
```

Mock and persona chat generation use the orchestrator's provider/model and seed.
A verified snapshot can be replayed with the provider offline; the chat response
reports zero new tokens and cost for that replay. Direct generator results expose
`replayed` so applications can distinguish cache reuse from a provider call. The
local admin HTTP handlers use the same policy from persisted workspace settings.
Hosted billing controls remain separate.

#### Persisted workspace policy for local admin generation (unreleased)

The local/self-hosted admin chat, mock and persona generation handlers accept a
`workspace_id` and share the workspace API's live registry. Set a policy with
`PUT /__mockforge/workspaces/<workspace_id>`:

```json
{
  "ai_mode": "generate_once_freeze",
  "ai_generation": {
    "seed": 42,
    "deterministic_mode": {
      "freeze_format": "json",
      "freeze_directory": "fixtures/ai",
      "track_metadata": false
    }
  }
}
```

Omitted fields retain existing overrides; explicit `null` removes an override.
Disabled workspaces return 403 without generation. Live registry policy takes
precedence. For a workspace absent from that registry, the handlers fall back to
a persisted local workspace file. By default these files are in
`./workspaces/<workspace_id>.yaml`. Add the following inside that workspace's
`config` section:

```yaml
config:
  ai_mode: generate_once_freeze
  ai_generation:
    seed: 42
    deterministic_mode:
      enabled: false
      freeze_format: json
      freeze_directory: fixtures/ai
      track_metadata: false
```

`generate_once_freeze` requests replay even with auto-freeze disabled. The
workspace seed overrides the server seed; omitting it inherits the server seed
or `MOCKFORGE_AI_SEED`. Provider/model settings come from the server's configured
MockAI instance, or its trusted authoring defaults when runtime MockAI is disabled. This workspace policy does not store API keys. With no workspace
mode override, the server's AI mode applies. Snapshot settings and mode survive
workspace save/reload and the legacy workspace export/import path.

The CLI forwards `mockai.intelligent_behavior` settings to authoring even with
`mockai.enabled: false`. This keeps the chosen local/provider model available to
authenticated authoring endpoints without enabling runtime AI for mock requests.
Disable Reality presets when selecting individual subsystem settings:

```yaml
reality:
  enabled: false
mockai:
  enabled: false
  intelligent_behavior:
    enabled: false
    behavior_model:
      llm_provider: ollama
      model: llama3.2
      seed: 42
```

Applications embedding the admin server can use
`start_admin_server_notify_with_authoring` with `AiAuthoringDefaults`. Existing
admin-server startup functions retain their signatures and defaults. An active
MockAI instance takes precedence so live configuration updates still apply.

Missing workspaces return 404, malformed workspace files return 500, and invalid
workspace IDs return 400 before any provider request. Chat uses its conversation's
workspace when the request omits one and rejects a conflicting workspace ID.
These local admin routes are separate from the hosted registry's metered AI
pipeline; this change does not activate hosted platform credentials.

This freezes the generated specification or persona, including any examples
already present. It does not turn schema-driven random response generation into
fixed response examples. For byte-identical HTTP assertions, commit explicit
response data and disable or pin runtime templates, clocks and stateful behavior.

### Seeded Generation (#852)

For the public authoring and offline CI workflow, see
[Bake AI responses for offline CI](baked-fixtures.md). It freezes actual response
data, including arrays and scalar values, into a static serve configuration.

`deterministic_mode` freezes AI output *after* generation. For
repeatability *during* generation, use a sampling seed. Provider support
and runtime changes can still affect the generated output:

```yaml
# mockforge.yaml: merge into your existing MockAI provider configuration
mockai:
  intelligent_behavior:
    behavior_model:
      seed: 42
```

For AI authoring services using `LlmClient`, set `BehaviorModelConfig.seed`
or use `MOCKFORGE_AI_SEED=42`. A seed on the typed `LlmGenerationRequest`
overrides the client configuration and environment. Resolution order:
request > client config > `MOCKFORGE_AI_SEED`. An HTTP authoring endpoint
must explicitly expose and forward a request seed to support that override;
adding an arbitrary `seed` field does not enable it.

Provider support:

| Provider | Seed support |
|----------|--------------|
| OpenAI | native (`seed` field) |
| OpenAI-compatible (vLLM, llama.cpp, LM Studio) | usually native |
| Ollama | native (`options.seed`) |
| Anthropic | none — pin `temperature: 0` instead; identical prompts are typically stable but not guaranteed byte-identical |

**CI reproducibility:** a seed improves repeatability; MockForge does not
guarantee byte-identical output from a live provider. For example,
[OpenAI documents seed sampling as best effort](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)
and explicitly does not guarantee determinism. Pin the model version,
prompt, seed, and generation settings when authoring fixtures, and review
changes when upgrading a provider or model.

For stable assertions, generate once and bake the result to YAML/JSON
fixtures (`auto_freeze`, above). Commit those artifacts and configure CI
to serve the committed fixtures without calling the provider or regenerating
them. Frozen data stays unchanged until you deliberately replace it;
response templates, timestamps, randomness, and stateful behavior must also
be disabled or pinned if your tests require byte-identical HTTP responses.
This workflow complements the AI generation rollout tracked in #842.

### System Coherence Validation

Ensures all generated components are coherent:

- Personas match endpoints
- Lifecycles match entities
- Scenarios match personas
- WebSocket topics match real-time features

## Configuration

### Enable System Generation

```yaml
# mockforge.yaml
ai_studio:
  system_generation:
    enabled: true
    default_output_formats:
      - openapi
      - personas
      - lifecycles
      - scenarios
```

### Output Format Selection

Selecting specific output formats (for example OpenAPI, personas, lifecycles, GraphQL) is planned; there is no CLI option for it yet.

## Best Practices

1. **Be Specific**: Detailed descriptions yield better results
2. **Mention Roles**: Include user roles (drivers, riders, admins)
3. **Specify Features**: Mention real-time, payments, etc.
4. **Review Generated**: Always review and refine generated artifacts
5. **Iterate**: Generate multiple versions and compare

## Real-World Examples

### E-Commerce Platform

**Input:**
```
E-commerce platform with products, shopping cart, orders, payments, 
inventory management, shipping, and customer reviews.
```

**Generated:**
- 25 REST endpoints
- 5 personas (customer, admin, vendor, support, reviewer)
- 8 lifecycle states (order, payment, shipping)
- WebSocket topics (order_updates, inventory_alerts)
- Payment failure scenarios
- Inventory depletion chaos profiles

### SaaS Platform

**Input:**
```
SaaS platform with users, subscriptions, billing, teams, projects, 
tasks, and real-time collaboration.
```

**Generated:**
- 30 REST endpoints
- 6 personas (admin, owner, member, viewer, billing_admin, support)
- 10 lifecycle states (subscription, project, task)
- WebSocket topics (collaboration_updates, task_updates)
- Billing failure scenarios
- Subscription lifecycle chaos profiles

## Related Documentation

- [AI Studio](llm-studio.md) - AI features overview
- [API Architecture Critique](api-architecture-critique.md) - API analysis
- [Behavioral Simulation](behavioral-simulation.md) - AI behavioral simulation

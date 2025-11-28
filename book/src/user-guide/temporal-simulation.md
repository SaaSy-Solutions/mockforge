# Temporal Simulation (Time Travel)

Temporal Simulation allows you to control time in your mock environment, enabling time-based data mutations, scheduled events, and time-travel debugging. Test time-dependent behavior without waiting for real time to pass.

## Overview

Time travel in MockForge works through a **virtual clock** that can be:

- **Enabled/disabled** at runtime
- **Set** to any specific point in time
- **Advanced** by arbitrary durations instantly
- **Scaled** to run faster or slower than real time

When time travel is enabled, all time-related features use the virtual clock instead of the system clock.

## Quick Start

### Enable Time Travel

```yaml
# config.yaml
core:
  time_travel:
    enabled: true
    initial_time: "2025-01-01T00:00:00Z"
    scale_factor: 1.0
    enable_scheduling: true
```

### Control Time via CLI

```bash
# Get time travel status
mockforge time status

# Enable time travel at a specific time
mockforge time enable --time "2025-01-01T00:00:00Z"

# Advance time by 1 month (instantly!)
mockforge time advance 1month

# Advance time by 2 hours
mockforge time advance 2h

# Set time to a specific point
mockforge time set "2025-06-01T12:00:00Z"

# Reset to real time
mockforge time reset
```

### Use Time-Based Templates

Time-aware template tokens automatically use the virtual clock:

```json
{
  "timestamp": "{{now}}",
  "expires_at": "{{now+1h}}",
  "created_at": "{{now-30m}}"
}
```

## Virtual Clock

The virtual clock is the core of temporal simulation. It provides:

### Basic Operations

```rust
use mockforge_core::time_travel::VirtualClock;

let clock = VirtualClock::new();

// Enable and set time
clock.enable_and_set(DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")?);

// Advance time
clock.advance(Duration::from_secs(3600)); // Advance 1 hour

// Get current virtual time
let now = clock.now();

// Disable (return to real time)
clock.disable();
```

### Time Scale

Run time faster or slower than real time:

```bash
# Run at 2x speed
mockforge time scale 2.0

# Run at 0.5x speed (half speed)
mockforge time scale 0.5
```

## Cron Scheduler

Schedule recurring events using cron expressions:

### Create Cron Job

```bash
# Via CLI
mockforge time cron create \
  --schedule "0 */6 * * *" \
  --action "callback" \
  --callback-url "http://localhost:3000/api/cleanup"

# Via API
curl -X POST http://localhost:9080/__mockforge/time-travel/cron \
  -H "Content-Type: application/json" \
  -d '{
    "schedule": "0 */6 * * *",
    "action": {
      "type": "callback",
      "url": "http://localhost:3000/api/cleanup"
    },
    "enabled": true
  }'
```

### Cron Expression Format

```
┌───────────── minute (0 - 59)
│ ┌───────────── hour (0 - 23)
│ │ ┌───────────── day of month (1 - 31)
│ │ │ ┌───────────── month (1 - 12)
│ │ │ │ ┌───────────── day of week (0 - 6) (Sunday to Saturday)
│ │ │ │ │
* * * * *
```

**Examples:**
- `0 */6 * * *` - Every 6 hours
- `0 0 * * *` - Daily at midnight
- `*/15 * * * *` - Every 15 minutes
- `0 9 * * 1-5` - Weekdays at 9 AM

### List Cron Jobs

```bash
# Via CLI
mockforge time cron list

# Via API
curl http://localhost:9080/__mockforge/time-travel/cron
```

## Mutation Rules

Automatically mutate data based on time triggers:

### Interval-Based Mutations

Mutate data at regular intervals:

```bash
# Create mutation rule
mockforge time mutation create \
  --entity "orders" \
  --trigger "interval:1h" \
  --operation "update_status" \
  --field "status" \
  --value "shipped"

# Via API
curl -X POST http://localhost:9080/__mockforge/time-travel/mutations \
  -H "Content-Type: application/json" \
  -d '{
    "entity": "orders",
    "trigger": {
      "type": "interval",
      "duration": "1h"
    },
    "operation": {
      "type": "update_status",
      "field": "status",
      "value": "shipped"
    }
  }'
```

### Time-Based Mutations

Mutate data at specific times:

```json
{
  "entity": "tokens",
  "trigger": {
    "type": "at_time",
    "time": "2025-01-01T12:00:00Z"
  },
  "operation": {
    "type": "set",
    "field": "expired",
    "value": true
  }
}
```

### Field Threshold Mutations

Mutate when a field reaches a threshold:

```json
{
  "entity": "orders",
  "trigger": {
    "type": "field_threshold",
    "field": "age_days",
    "operator": ">=",
    "value": 30
  },
  "operation": {
    "type": "set",
    "field": "status",
    "value": "archived"
  }
}
```

## Scheduled Responses

Schedule responses to be sent at specific times:

```bash
# Schedule a response for 30 minutes from now
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -H "Content-Type: application/json" \
  -d '{
    "trigger_time": "+30m",
    "path": "/api/notifications",
    "method": "POST",
    "body": {"event": "token_expired"},
    "status": 401
  }'
```

## VBR Integration

Temporal simulation integrates with the VBR Engine for time-based data mutations:

### Snapshot with Time Travel

Create snapshots that include time travel state:

```rust
use mockforge_vbr::VbrEngine;

// Create snapshot with time travel state
engine.create_snapshot_with_time_travel(
    "snapshot1",
    Some("Description".to_string()),
    "./snapshots",
    &clock
).await?;

// Restore snapshot with time travel state
engine.restore_snapshot_with_time_travel(
    "snapshot1",
    "./snapshots",
    &clock
).await?;
```

### Mutation Rules in VBR

VBR automatically executes mutation rules based on virtual time:

```yaml
vbr:
  entities:
    - name: orders
      mutation_rules:
        - trigger: "interval:1h"
          operation: "update_status"
          field: "status"
          value: "processing"
```

## Admin API

### Time Travel Status

```http
GET /__mockforge/time-travel/status
```

Response:
```json
{
  "enabled": true,
  "virtual_time": "2025-01-15T10:30:00Z",
  "real_time": "2025-01-01T10:30:00Z",
  "scale_factor": 1.0
}
```

### Advance Time

```http
POST /__mockforge/time-travel/advance
Content-Type: application/json

{
  "duration": "2h"  # or "1month", "30m", etc.
}
```

### Set Time

```http
PUT /__mockforge/time-travel/time
Content-Type: application/json

{
  "time": "2025-06-01T12:00:00Z"
}
```

### Enable/Disable

```http
POST /__mockforge/time-travel/enable
Content-Type: application/json

{
  "time": "2025-01-01T00:00:00Z"  # Optional initial time
}
```

```http
POST /__mockforge/time-travel/disable
```

## CLI Commands

### Time Control

```bash
# Status
mockforge time status

# Enable
mockforge time enable [--time "2025-01-01T00:00:00Z"]

# Disable
mockforge time disable

# Advance
mockforge time advance <duration>  # e.g., "1month", "2h", "30m"

# Set
mockforge time set <time>  # ISO 8601 format

# Scale
mockforge time scale <factor>  # e.g., 2.0 for 2x speed

# Reset
mockforge time reset
```

### Cron Jobs

```bash
# List
mockforge time cron list

# Create
mockforge time cron create --schedule "<cron>" --action "<action>"

# Get
mockforge time cron get <id>

# Update
mockforge time cron update <id> --enabled false

# Delete
mockforge time cron delete <id>
```

### Mutation Rules

```bash
# List
mockforge time mutation list

# Create
mockforge time mutation create --entity "<entity>" --trigger "<trigger>" --operation "<operation>"

# Get
mockforge time mutation get <id>

# Update
mockforge time mutation update <id> --enabled false

# Delete
mockforge time mutation delete <id>
```

## Use Cases

### Token Expiration

Test token expiration without waiting:

```bash
# Create token that expires in 1 hour
mockforge time enable --time "2025-01-01T00:00:00Z"

# Advance 1 hour
mockforge time advance 1h

# Token is now expired
```

### Session Timeouts

Test session timeout behavior:

```yaml
vbr:
  entities:
    - name: sessions
      ttl_seconds: 3600  # 1 hour
      aging_enabled: true
```

### Scheduled Events

Test scheduled notifications:

```bash
# Schedule notification for 1 day from now
mockforge time cron create \
  --schedule "0 0 * * *" \
  --action "callback" \
  --callback-url "http://localhost:3000/api/send-daily-report"
```

### Data Aging

Test data that changes over time:

```bash
# Create mutation rule to age orders
mockforge time mutation create \
  --entity "orders" \
  --trigger "interval:1d" \
  --operation "increment" \
  --field "age_days"
```

## Best Practices

1. **Start with Simple Scenarios**: Begin with basic time advancement before using cron or mutations
2. **Use Snapshots**: Save important time states for quick restoration
3. **Test Edge Cases**: Test behavior at midnight, month boundaries, etc.
4. **Monitor Performance**: Time-based features add minimal overhead
5. **Combine with VBR**: Use VBR entities with time-based mutations for realistic scenarios

## Troubleshooting

### Time Not Advancing

- Ensure time travel is enabled: `mockforge time status`
- Check that scheduling is enabled in configuration
- Verify cron jobs are enabled

### Mutations Not Executing

- Check mutation rule is enabled
- Verify trigger conditions are met
- Review server logs for errors

### Cron Jobs Not Running

- Ensure cron scheduler background task is running
- Check cron expression is valid
- Verify job is enabled

## Related Documentation

- [VBR Engine](vbr-engine.md) - State management with time-based mutations
- [Scenario State Machines](scenario-state-machines.md) - Time-based state transitions
- [Configuration Guide](../configuration/files.md) - Complete configuration reference


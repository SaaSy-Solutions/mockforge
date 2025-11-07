# Time Travel / Temporal Testing

MockForge includes powerful time travel capabilities for testing time-dependent behavior without waiting for real time to pass. This feature is perfect for testing scenarios like:

- Token expiration and renewal
- Session timeouts
- Time-based state transitions
- Scheduled events and notifications
- Data that changes over time
- Rate limiting based on time windows

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Virtual Clock](#virtual-clock)
- [Scheduled Responses](#scheduled-responses)
- [Cron Scheduler](#cron-scheduler)
- [Mutation Rules](#mutation-rules)
- [Template Integration](#template-integration)
- [Admin API](#admin-api)
- [CLI Commands](#cli-commands)
- [Use Cases](#use-cases)
- [Examples](#examples)

## Overview

Time travel in MockForge works through a **virtual clock** that can be:
- **Enabled/disabled** at runtime
- **Set** to any specific point in time
- **Advanced** by arbitrary durations instantly
- **Scaled** to run faster or slower than real time

When time travel is enabled, all time-related features in MockForge use the virtual clock instead of the system clock:
- Template tokens like `{{now}}`, `{{now+1h}}`, etc.
- Scheduled responses
- Time-based logging and metrics

## Quick Start

### 1. Enable Time Travel via Configuration

```yaml
# config.yaml
core:
  time_travel:
    enabled: true
    initial_time: "2025-01-01T00:00:00Z"  # Optional: start at specific time
    scale_factor: 1.0                      # 1.0 = normal speed
    enable_scheduling: true

http:
  response_template_expand: true  # Required for template tokens
```

### 2. Start MockForge

```bash
mockforge serve --config config.yaml --admin
```

### 3. Use Time-Based Templates

Create an endpoint that returns current time:

```json
{
  "timestamp": "{{now}}",
  "expires_at": "{{now+1h}}",
  "created_at": "{{now-30m}}"
}
```

### 4. Control Time via CLI (Recommended)

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

# Set time scale (2x speed)
mockforge time scale 2.0

# Save current state as a scenario
mockforge time save "1-month-later" --description "Scenario after 1 month"

# Load a saved scenario
mockforge time load "1-month-later"

# List all saved scenarios
mockforge time list

# Reset to real time
mockforge time reset
```

### 5. Control Time via Admin API

```bash
# Get time travel status
curl http://localhost:9080/__mockforge/time-travel/status

# Advance time by 2 hours
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "2h"}'

# Advance time by 1 month
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "1month"}'

# Schedule a response for 30 minutes from now
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -H "Content-Type: application/json" \
  -d '{
    "trigger_time": "+30m",
    "body": {"event": "token_expired"},
    "status": 401
  }'
```

## Configuration

### Time Travel Configuration

```yaml
core:
  time_travel:
    # Whether time travel is enabled at startup
    enabled: true

    # Initial virtual time (ISO 8601 format)
    # If not specified, uses current time when enabled
    initial_time: "2025-01-01T00:00:00Z"

    # Time scale factor
    # 1.0 = real time (default)
    # 2.0 = 2x speed (1 real second = 2 virtual seconds)
    # 0.5 = half speed (1 real second = 0.5 virtual seconds)
    scale_factor: 1.0

    # Enable scheduled responses
    enable_scheduling: true
```

### Enable Template Expansion

Time travel requires template expansion to be enabled:

```yaml
http:
  response_template_expand: true
```

## Virtual Clock

The virtual clock is the core of time travel functionality.

### Enabling/Disabling

```bash
# Enable time travel at current time
curl -X POST http://localhost:9080/__mockforge/time-travel/enable

# Enable at specific time
curl -X POST http://localhost:9080/__mockforge/time-travel/enable \
  -H "Content-Type: application/json" \
  -d '{"time": "2025-06-01T12:00:00Z"}'

# Disable time travel (return to real time)
curl -X POST http://localhost:9080/__mockforge/time-travel/disable
```

### Advancing Time

```bash
# Advance by duration
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "2h"}'

# Supported units: s (seconds), m (minutes), h (hours), d (days)
```

### Time Scale

Control how fast virtual time progresses relative to real time:

```bash
# Set 2x speed
curl -X POST http://localhost:9080/__mockforge/time-travel/scale \
  -H "Content-Type: application/json" \
  -d '{"scale": 2.0}'

# Set half speed
curl -X POST http://localhost:9080/__mockforge/time-travel/scale \
  -H "Content-Type: application/json" \
  -d '{"scale": 0.5}'
```

### Checking Status

```bash
curl http://localhost:9080/__mockforge/time-travel/status
```

Response:

```json
{
  "enabled": true,
  "current_time": "2025-01-01T14:30:00Z",
  "scale_factor": 1.0,
  "real_time": "2025-01-15T10:22:33Z"
}
```

## Scheduled Responses

Schedule responses to be returned at specific virtual times.

### Scheduling a Response

```bash
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -H "Content-Type: application/json" \
  -d '{
    "trigger_time": "+1h",
    "body": {"message": "Token expired"},
    "status": 401,
    "headers": {
      "X-Error-Code": "TOKEN_EXPIRED"
    },
    "name": "token_expiry"
  }'
```

#### Trigger Time Formats

- **Relative time**: `+1h`, `-30m`, `+2d`, etc.
- **Absolute time**: `2025-01-01T15:00:00Z` (ISO 8601)

### Repeating Responses

```bash
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -H "Content-Type: application/json" \
  -d '{
    "trigger_time": "+5m",
    "body": {"event": "heartbeat"},
    "status": 200,
    "repeat": {
      "interval": "PT5M",
      "max_count": 10
    }
  }'
```

### Managing Scheduled Responses

```bash
# List all scheduled responses
curl http://localhost:9080/__mockforge/time-travel/scheduled

# Cancel a specific response
curl -X DELETE http://localhost:9080/__mockforge/time-travel/scheduled/{id}

# Clear all scheduled responses
curl -X POST http://localhost:9080/__mockforge/time-travel/scheduled/clear
```

## Cron Scheduler

The cron scheduler allows you to schedule recurring events using cron expressions. It works alongside the ResponseScheduler and integrates with the virtual clock.

### Creating Cron Jobs

```bash
# Create a cron job that runs every day at 3am
curl -X POST http://localhost:9080/__mockforge/time-travel/cron \
  -H "Content-Type: application/json" \
  -d '{
    "id": "daily-cleanup",
    "name": "Daily Cleanup",
    "schedule": "0 3 * * *",
    "description": "Runs daily cleanup at 3am",
    "action_type": "callback",
    "action_metadata": {}
  }'

# Create a cron job that sends a response every hour
curl -X POST http://localhost:9080/__mockforge/time-travel/cron \
  -H "Content-Type: application/json" \
  -d '{
    "id": "hourly-report",
    "name": "Hourly Report",
    "schedule": "0 * * * *",
    "action_type": "response",
    "action_metadata": {
      "body": {"report": "hourly_data"},
      "status": 200,
      "headers": {"Content-Type": "application/json"}
    }
  }'
```

### Cron Expression Format

Standard 5-field cron format:
```
┌───────────── minute (0 - 59)
│ ┌───────────── hour (0 - 23)
│ │ ┌───────────── day of month (1 - 31)
│ │ │ ┌───────────── month (1 - 12)
│ │ │ │ ┌───────────── day of week (0 - 6) (Sunday to Saturday)
│ │ │ │ │
* * * * *
```

Examples:
- `0 3 * * *` - Every day at 3:00 AM
- `*/15 * * * *` - Every 15 minutes
- `0 0 * * 0` - Every Sunday at midnight
- `0 9-17 * * 1-5` - Every hour from 9 AM to 5 PM on weekdays

### Managing Cron Jobs

```bash
# List all cron jobs
curl http://localhost:9080/__mockforge/time-travel/cron

# Get a specific cron job
curl http://localhost:9080/__mockforge/time-travel/cron/{id}

# Enable/disable a cron job
curl -X POST http://localhost:9080/__mockforge/time-travel/cron/{id}/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'

# Delete a cron job
curl -X DELETE http://localhost:9080/__mockforge/time-travel/cron/{id}
```

### CLI Commands for Cron Jobs

```bash
# List all cron jobs
mockforge time cron list

# Create a cron job
mockforge time cron create \
  --id "daily-cleanup" \
  --name "Daily Cleanup" \
  --schedule "0 3 * * *" \
  --action-type "callback" \
  --action-metadata ./action.json

# Get a specific cron job
mockforge time cron get daily-cleanup

# Enable/disable a cron job
mockforge time cron enable daily-cleanup
mockforge time cron disable daily-cleanup

# Delete a cron job
mockforge time cron delete daily-cleanup
```

## Mutation Rules

Mutation rules automatically modify VBR entity data based on time triggers. This is useful for simulating data aging, status changes, and other time-based data evolution.

### Creating Mutation Rules

```bash
# Create a rule that increments a counter every hour
curl -X POST http://localhost:9080/__mockforge/time-travel/mutations \
  -H "Content-Type: application/json" \
  -d '{
    "id": "hourly-counter",
    "entity_name": "User",
    "trigger": {
      "type": "interval",
      "duration_seconds": 3600
    },
    "operation": {
      "type": "increment",
      "field": "login_count",
      "amount": 1.0
    },
    "description": "Increment login count every hour"
  }'

# Create a rule that runs at a specific time daily
curl -X POST http://localhost:9080/__mockforge/time-travel/mutations \
  -H "Content-Type: application/json" \
  -d '{
    "id": "daily-reset",
    "entity_name": "User",
    "trigger": {
      "type": "attime",
      "hour": 3,
      "minute": 0
    },
    "operation": {
      "type": "set",
      "field": "status",
      "value": "active"
    }
  }'
```

### Trigger Types

#### Interval Trigger
Executes after a duration has elapsed:
```json
{
  "type": "interval",
  "duration_seconds": 3600
}
```

#### At Time Trigger
Executes at a specific time each day:
```json
{
  "type": "attime",
  "hour": 3,
  "minute": 0
}
```

#### Field Threshold Trigger
Executes when a field value reaches a threshold:
```json
{
  "type": "fieldthreshold",
  "field": "balance",
  "threshold": 0,
  "operator": "lte"
}
```

### Operation Types

#### Set Operation
Set a field to a specific value:
```json
{
  "type": "set",
  "field": "status",
  "value": "expired"
}
```

#### Increment Operation
Increment a numeric field:
```json
{
  "type": "increment",
  "field": "count",
  "amount": 1.0
}
```

#### Decrement Operation
Decrement a numeric field:
```json
{
  "type": "decrement",
  "field": "balance",
  "amount": 10.0
}
```

#### Update Status Operation
Update a status field:
```json
{
  "type": "updatestatus",
  "status": "inactive"
}
```

### Managing Mutation Rules

```bash
# List all mutation rules
curl http://localhost:9080/__mockforge/time-travel/mutations

# Get a specific mutation rule
curl http://localhost:9080/__mockforge/time-travel/mutations/{id}

# Enable/disable a mutation rule
curl -X POST http://localhost:9080/__mockforge/time-travel/mutations/{id}/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'

# Delete a mutation rule
curl -X DELETE http://localhost:9080/__mockforge/time-travel/mutations/{id}
```

### CLI Commands for Mutation Rules

```bash
# List all mutation rules
mockforge time mutation list

# Create a mutation rule
mockforge time mutation create \
  --id "hourly-counter" \
  --entity User \
  --trigger-type interval \
  --trigger-config ./trigger.json \
  --operation-type increment \
  --operation-config ./operation.json

# Get a specific mutation rule
mockforge time mutation get hourly-counter

# Enable/disable a mutation rule
mockforge time mutation enable hourly-counter
mockforge time mutation disable hourly-counter

# Delete a mutation rule
mockforge time mutation delete hourly-counter
```

## VBR Snapshot Integration

VBR snapshots can optionally include time travel state (cron jobs and mutation rules) for complete state restoration.

### Creating Snapshots with Time Travel State

When creating a VBR snapshot, you can include time travel state:

```rust
use mockforge_vbr::{VbrEngine, TimeTravelSnapshotState};
use chrono::Utc;

let engine = VbrEngine::new(config).await?;

// Create time travel state
let time_travel_state = TimeTravelSnapshotState {
    enabled: true,
    current_time: Some(Utc::now()),
    scale_factor: 1.0,
    cron_jobs: vec![/* serialized cron jobs */],
    mutation_rules: vec![/* serialized mutation rules */],
};

// Create snapshot with time travel state
let metadata = engine
    .create_snapshot_with_time_travel(
        "snapshot-with-time-travel",
        Some("Snapshot with time travel state".to_string()),
        "./snapshots",
        true,
        Some(time_travel_state),
    )
    .await?;
```

### Restoring Snapshots with Time Travel State

When restoring a snapshot, you can restore the time travel state:

```rust
// Restore snapshot with time travel state
engine
    .restore_snapshot_with_time_travel(
        "snapshot-with-time-travel",
        "./snapshots",
        true,
        Some(|state| {
            // Restore time travel state
            // This callback receives the TimeTravelSnapshotState
            // and can restore it to the TimeTravelManager
            Ok(())
        }),
    )
    .await?;
```

## Template Integration

When time travel is enabled, all time-related template tokens use the virtual clock.

### Time Tokens

```text
{{now}}           # Current virtual time (ISO 8601)
{{now+1h}}        # 1 hour from now
{{now-30m}}       # 30 minutes ago
{{now+2d}}        # 2 days from now
{{now-1d}}        # 1 day ago
```

### Example Response Template

```json
{
  "user_id": "{{uuid}}",
  "token": "{{uuid}}",
  "issued_at": "{{now}}",
  "expires_at": "{{now+1h}}",
  "refresh_before": "{{now+50m}}",
  "session_data": {
    "created": "{{now}}",
    "last_activity": "{{now-5m}}"
  }
}
```

### Using in OpenAPI Specs

```yaml
paths:
  /auth/token:
    get:
      responses:
        '200':
          description: Authentication token
          content:
            application/json:
              example:
                token: "{{uuid}}"
                expires_at: "{{now+1h}}"
```

## CLI Commands

The `mockforge time` command provides a convenient CLI interface for controlling time travel:

### Basic Commands

```bash
# Show current status
mockforge time status

# Enable time travel
mockforge time enable [--time TIME] [--scale FACTOR]

# Disable time travel
mockforge time disable

# Advance time
mockforge time advance <duration>

# Set time to specific point
mockforge time set <time>

# Set time scale
mockforge time scale <factor>

# Reset to real time
mockforge time reset
```

### Duration Formats

Supported duration formats for `advance`:
- `10s`, `30m`, `2h`, `1d` - Standard units
- `1week`, `1 week`, `2weeks` - Weeks (7 days)
- `1month`, `2months` - Months (approximate: 30 days)
- `1year`, `2years` - Years (approximate: 365 days)
- `+1h`, `+1 week`, `+2d` - With + prefix (optional)

Examples:
```bash
mockforge time advance 1month    # Advance by 1 month
mockforge time advance 2h        # Advance by 2 hours
mockforge time advance 30m       # Advance by 30 minutes
mockforge time advance +1 week   # Advance by 1 week (with + prefix)
mockforge time advance 1week      # Advance by 1 week
```

### Cron Job Management

```bash
# List all cron jobs
mockforge time cron list

# Create a cron job
mockforge time cron create \
  --id "daily-cleanup" \
  --name "Daily Cleanup" \
  --schedule "0 3 * * *" \
  --action-type "callback" \
  --action-metadata ./action.json

# Get a specific cron job
mockforge time cron get daily-cleanup

# Enable/disable a cron job
mockforge time cron enable daily-cleanup
mockforge time cron disable daily-cleanup

# Delete a cron job
mockforge time cron delete daily-cleanup
```

### Mutation Rule Management

```bash
# List all mutation rules
mockforge time mutation list

# Create a mutation rule
mockforge time mutation create \
  --id "hourly-counter" \
  --entity User \
  --trigger-type interval \
  --trigger-config ./trigger.json \
  --operation-type increment \
  --operation-config ./operation.json

# Get a specific mutation rule
mockforge time mutation get hourly-counter

# Enable/disable a mutation rule
mockforge time mutation enable hourly-counter
mockforge time mutation disable hourly-counter

# Delete a mutation rule
mockforge time mutation delete hourly-counter
```

### Scenario Management

Save and load time travel states for repeatable testing:

```bash
# Save current state
mockforge time save <name> [--description TEXT] [--output PATH]

# Load a saved scenario
mockforge time load <name>

# List all saved scenarios
mockforge time list [--dir PATH]
```

Example workflow:
```bash
# 1. Enable time travel and set initial time
mockforge time enable --time "2025-01-01T00:00:00Z"

# 2. Advance to 1 month later
mockforge time advance 1month

# 3. Save this state
mockforge time save "1-month-later" --description "State after 1 month"

# 4. Later, load it again
mockforge time load "1-month-later"
```

Scenarios are saved as JSON files in `./scenarios/` by default.

## Admin API

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/__mockforge/time-travel/status` | GET | Get time travel status |
| `/__mockforge/time-travel/enable` | POST | Enable time travel |
| `/__mockforge/time-travel/disable` | POST | Disable time travel |
| `/__mockforge/time-travel/advance` | POST | Advance time by duration |
| `/__mockforge/time-travel/scale` | POST | Set time scale factor |
| `/__mockforge/time-travel/reset` | POST | Reset to real time |
| `/__mockforge/time-travel/schedule` | POST | Schedule a response |
| `/__mockforge/time-travel/scheduled` | GET | List scheduled responses |
| `/__mockforge/time-travel/scheduled/{id}` | DELETE | Cancel scheduled response |
| `/__mockforge/time-travel/scheduled/clear` | POST | Clear all scheduled responses |
| `/__mockforge/time-travel/scenario/save` | POST | Save current state as scenario |
| `/__mockforge/time-travel/scenario/load` | POST | Load a scenario |
| `/__mockforge/time-travel/cron` | GET | List all cron jobs |
| `/__mockforge/time-travel/cron` | POST | Create a cron job |
| `/__mockforge/time-travel/cron/{id}` | GET | Get a specific cron job |
| `/__mockforge/time-travel/cron/{id}` | DELETE | Delete a cron job |
| `/__mockforge/time-travel/cron/{id}/enable` | POST | Enable/disable a cron job |
| `/__mockforge/time-travel/mutations` | GET | List all mutation rules |
| `/__mockforge/time-travel/mutations` | POST | Create a mutation rule |
| `/__mockforge/time-travel/mutations/{id}` | GET | Get a specific mutation rule |
| `/__mockforge/time-travel/mutations/{id}` | DELETE | Delete a mutation rule |
| `/__mockforge/time-travel/mutations/{id}/enable` | POST | Enable/disable a mutation rule |

### Request/Response Schemas

See [Admin API Reference](#admin-api-reference) for detailed schemas.

## Use Cases

### 1. Testing Token Expiry

Test how your application handles expired tokens:

```bash
# Start with fresh token
curl http://localhost:3000/auth/login
# Response: {"token": "abc123", "expires_at": "2025-01-01T01:00:00Z"}

# Advance time past expiry
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "2h"}'

# Try to use token - should be expired
curl -H "Authorization: Bearer abc123" http://localhost:3000/api/data
```

### 2. Testing Scheduled Events with Cron Jobs

Test recurring events using cron scheduler:

```bash
# Create a cron job that runs every hour
curl -X POST http://localhost:9080/__mockforge/time-travel/cron \
  -H "Content-Type: application/json" \
  -d '{
    "id": "hourly-check",
    "name": "Hourly Health Check",
    "schedule": "0 * * * *",
    "action_type": "response",
    "action_metadata": {
      "body": {"status": "healthy"},
      "status": 200
    }
  }'

# Advance time by 1 hour to trigger the job
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "1h"}'
```

### 3. Testing Data Aging with Mutation Rules

Simulate data that changes over time:

```bash
# Create a mutation rule that increments a counter every hour
curl -X POST http://localhost:9080/__mockforge/time-travel/mutations \
  -H "Content-Type: application/json" \
  -d '{
    "id": "hourly-login-count",
    "entity_name": "User",
    "trigger": {
      "type": "interval",
      "duration_seconds": 3600
    },
    "operation": {
      "type": "increment",
      "field": "login_count",
      "amount": 1.0
    }
  }'

# Advance time by 1 hour
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "1h"}'

# Check that login_count was incremented
curl http://localhost:3000/api/users/user1
```

### 4. Testing Scheduled Events

Test event-based systems:

```bash
# Schedule a notification for 5 minutes from now
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -d '{
    "trigger_time": "+5m",
    "body": {"event": "reminder", "message": "Meeting in 5 minutes"}
  }'

# Advance time
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "6m"}'

# Next request returns the scheduled response
curl http://localhost:3000/api/notifications
```

### 3. Testing Rate Limiting

Test time-window based rate limits:

```bash
# Make requests
curl http://localhost:3000/api/endpoint  # Success
curl http://localhost:3000/api/endpoint  # Success
curl http://localhost:3000/api/endpoint  # Rate limited

# Advance time past the window
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "1h"}'

# Should work again
curl http://localhost:3000/api/endpoint  # Success
```

### 4. Testing Session Timeouts

```bash
# Create session
curl http://localhost:3000/auth/login
# Response: {"session_id": "xyz", "expires_at": "{{now+30m}}"}

# Advance past timeout
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "31m"}'

# Session should be invalid
curl http://localhost:3000/api/data --cookie "session=xyz"
# Response: 401 Unauthorized
```

### 5. Testing Data Evolution

Test how data changes over time:

```bash
# Order status progression
curl http://localhost:3000/orders/123
# Response: {"status": "processing", "updated_at": "{{now}}"}

# Advance time
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "1h"}'

curl http://localhost:3000/orders/123
# Response: {"status": "shipped", "updated_at": "{{now}}"}
```

## Examples

### Complete Test Scenario

```bash
#!/bin/bash

# 1. Start MockForge with time travel
mockforge serve --config time-travel-demo.yaml --admin

# 2. Enable time travel at a known time (using CLI)
mockforge time enable --time "2025-01-01T00:00:00Z"

# Or using API
curl -X POST http://localhost:9080/__mockforge/time-travel/enable \
  -d '{"time": "2025-01-01T00:00:00Z"}'

# 3. Schedule multiple events
# Event at +1h
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -d '{
    "trigger_time": "+1h",
    "body": {"event": "hourly_sync"},
    "name": "hourly"
  }'

# Event at +30m
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -d '{
    "trigger_time": "+30m",
    "body": {"event": "token_refresh_needed"},
    "name": "token_refresh"
  }'

# 4. Advance time to trigger first event (using CLI)
mockforge time advance 35m

# Or using API
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "35m"}'

# 5. Next request should return scheduled response
curl http://localhost:3000/api/events
# Response: {"event": "token_refresh_needed"}

# 6. Continue advancing
mockforge time advance 30m

curl http://localhost:3000/api/events
# Response: {"event": "hourly_sync"}

# 7. Save scenario for future use
mockforge time save "test-scenario" --description "Complete test scenario"
```

### Integration Test Example

```python
import requests
import json

BASE_URL = "http://localhost:3000"
ADMIN_URL = "http://localhost:9080"

def test_token_expiry():
    # Enable time travel
    requests.post(f"{ADMIN_URL}/__mockforge/time-travel/enable",
                  json={"time": "2025-01-01T00:00:00Z"})

    # Get token
    response = requests.get(f"{BASE_URL}/auth/token")
    token = response.json()["token"]
    expires_at = response.json()["expires_at"]

    # Token should work initially
    response = requests.get(f"{BASE_URL}/api/data",
                           headers={"Authorization": f"Bearer {token}"})
    assert response.status_code == 200

    # Advance past expiry
    requests.post(f"{ADMIN_URL}/__mockforge/time-travel/advance",
                  json={"duration": "2h"})

    # Token should be expired
    response = requests.get(f"{BASE_URL}/api/data",
                           headers={"Authorization": f"Bearer {token}"})
    assert response.status_code == 401

    # Cleanup
    requests.post(f"{ADMIN_URL}/__mockforge/time-travel/disable")
```

## Admin API Reference

### Enable Time Travel

**POST** `/__mockforge/time-travel/enable`

Request:
```json
{
  "time": "2025-01-01T00:00:00Z",  // Optional: ISO 8601 format
  "scale": 1.0                      // Optional: time scale factor
}
```

Response:
```json
{
  "success": true,
  "status": {
    "enabled": true,
    "current_time": "2025-01-01T00:00:00Z",
    "scale_factor": 1.0,
    "real_time": "2025-01-15T10:30:00Z"
  }
}
```

### Advance Time

**POST** `/__mockforge/time-travel/advance`

Request:
```json
{
  "duration": "2h"  // Format: <number><unit> (s, m, h, d)
}
```

### Schedule Response

**POST** `/__mockforge/time-travel/schedule`

Request:
```json
{
  "trigger_time": "+1h",              // Relative (+1h) or absolute (ISO 8601)
  "body": {"message": "Hello"},       // JSON response body
  "status": 200,                      // HTTP status code
  "headers": {                        // Optional headers
    "X-Custom": "value"
  },
  "name": "my_schedule",              // Optional name
  "repeat": {                         // Optional repeat config
    "interval": "PT5M",               // ISO 8601 duration
    "max_count": 10                   // Max repetitions
  }
}
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "trigger_time": "2025-01-01T01:00:00Z"
}
```

## Tips and Best Practices

1. **Always enable template expansion**: Set `response_template_expand: true` in your HTTP config.

2. **Use relative times for scheduling**: Use `+1h` instead of absolute times for more flexible tests.

3. **Reset time travel between tests**: Call the reset endpoint to ensure clean state.

4. **Monitor scheduled responses**: Use the list endpoint to track what's scheduled.

5. **Use time scale carefully**: Time scale affects all time-based operations, including latency simulation.

6. **Combine with other features**: Time travel works great with:
   - Latency profiles
   - Failure injection
   - Request chaining
   - Data drift simulation

## Limitations

- Time travel only affects MockForge's internal clock, not system time
- External services will still use real time
- Some operations (like actual network delays) cannot be accelerated
- Scheduled responses are in-memory only (not persisted)

## Troubleshooting

**Time tokens not updating:**
- Ensure `response_template_expand: true` is set
- Check that time travel is enabled via the status endpoint

**Scheduled responses not triggering:**
- Verify the trigger time hasn't passed
- Check that enable_scheduling is true
- Ensure you're making requests after advancing time

**Virtual time not advancing:**
- Confirm time travel is enabled
- Check that advance requests are successful
- Verify the duration format is correct

## See Also

- [Template Expansion](./TEMPLATING.md)
- [Admin UI Guide](./ADMIN_UI.md)
- [Testing Guide](./TESTING.md)

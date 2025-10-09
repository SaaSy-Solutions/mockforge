# Time Travel Examples

This directory contains examples demonstrating MockForge's time travel / temporal testing capabilities.

## Quick Start

### 1. Start MockForge with Time Travel

```bash
# Start MockForge with time travel configuration
mockforge serve --config examples/time-travel-demo.yaml --admin

# Or with environment variables
MOCKFORGE_TIME_TRAVEL_ENABLED=true \
MOCKFORGE_TIME_TRAVEL_INITIAL_TIME="2025-01-01T00:00:00Z" \
mockforge serve --admin
```

### 2. Run the Demo Script

```bash
./examples/time-travel-demo.sh
```

This script demonstrates:
- Enabling time travel at a specific time
- Scheduling responses for future times
- Advancing time to trigger scheduled responses
- Using time scale to speed up time
- Resetting to real time

## Files

- **`time-travel-demo.yaml`** - Example configuration with time travel enabled
- **`time-travel-demo.sh`** - Interactive demo script showing all features
- **`README-time-travel.md`** - This file

## Manual Testing

### Enable Time Travel

```bash
# Enable at current time
curl -X POST http://localhost:9080/__mockforge/time-travel/enable

# Enable at specific time
curl -X POST http://localhost:9080/__mockforge/time-travel/enable \
  -H "Content-Type: application/json" \
  -d '{"time": "2025-01-01T00:00:00Z"}'
```

### Check Status

```bash
curl http://localhost:9080/__mockforge/time-travel/status | jq '.'
```

Example response:
```json
{
  "enabled": true,
  "current_time": "2025-01-01T00:00:00Z",
  "scale_factor": 1.0,
  "real_time": "2025-01-15T14:30:00Z"
}
```

### Advance Time

```bash
# Advance by 2 hours
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "2h"}'

# Advance by 30 minutes
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "30m"}'

# Advance by 1 day
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "1d"}'
```

### Schedule Responses

```bash
# Schedule a response for 1 hour from now
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

# Schedule a repeating response
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

### List Scheduled Responses

```bash
curl http://localhost:9080/__mockforge/time-travel/scheduled | jq '.'
```

### Cancel Scheduled Response

```bash
# Get the ID from the list above
curl -X DELETE http://localhost:9080/__mockforge/time-travel/scheduled/{id}
```

### Set Time Scale

```bash
# 2x speed (time passes twice as fast)
curl -X POST http://localhost:9080/__mockforge/time-travel/scale \
  -H "Content-Type: application/json" \
  -d '{"scale": 2.0}'

# Half speed
curl -X POST http://localhost:9080/__mockforge/time-travel/scale \
  -H "Content-Type: application/json" \
  -d '{"scale": 0.5}'

# Back to normal
curl -X POST http://localhost:9080/__mockforge/time-travel/scale \
  -H "Content-Type: application/json" \
  -d '{"scale": 1.0}'
```

### Reset Time Travel

```bash
# Disable and return to real time
curl -X POST http://localhost:9080/__mockforge/time-travel/reset
```

## Use Case Examples

### Testing Token Expiry

```bash
# 1. Enable time travel
curl -X POST http://localhost:9080/__mockforge/time-travel/enable \
  -d '{"time": "2025-01-01T00:00:00Z"}'

# 2. Get a token (with 1 hour expiry)
curl http://localhost:3000/auth/token
# Response: {"token": "abc123", "expires_at": "2025-01-01T01:00:00Z"}

# 3. Advance past expiry
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "2h"}'

# 4. Try to use expired token
curl -H "Authorization: Bearer abc123" http://localhost:3000/api/data
# Should return 401 Unauthorized
```

### Testing Session Timeout

```bash
# 1. Create session (30 minute timeout)
curl -X POST http://localhost:3000/auth/login \
  -d '{"username": "test", "password": "test"}'
# Response: {"session_id": "xyz", "expires_at": "{{now+30m}}"}

# 2. Advance time past timeout
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "31m"}'

# 3. Try to use session
curl --cookie "session=xyz" http://localhost:3000/api/data
# Should return 401 Unauthorized
```

### Testing Scheduled Events

```bash
# 1. Schedule notification
curl -X POST http://localhost:9080/__mockforge/time-travel/schedule \
  -d '{
    "trigger_time": "+10m",
    "body": {"event": "reminder", "message": "Meeting in 10 minutes"}
  }'

# 2. Advance time
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "11m"}'

# 3. Next request returns scheduled response
curl http://localhost:3000/api/notifications
```

### Testing Rate Limiting

```bash
# 1. Make requests until rate limited
for i in {1..5}; do
  curl http://localhost:3000/api/endpoint
done
# Last request returns 429 Too Many Requests

# 2. Advance past rate limit window
curl -X POST http://localhost:9080/__mockforge/time-travel/advance \
  -d '{"duration": "1h"}'

# 3. Should work again
curl http://localhost:3000/api/endpoint
# Returns 200 OK
```

## Template Examples

Use time-based templates in your responses:

```json
{
  "user_id": "{{uuid}}",
  "token": "{{uuid}}",
  "issued_at": "{{now}}",
  "expires_at": "{{now+1h}}",
  "refresh_before": "{{now+50m}}",
  "session_data": {
    "created": "{{now}}",
    "last_activity": "{{now-5m}}",
    "next_check": "{{now+15m}}"
  }
}
```

### Available Time Tokens

- `{{now}}` - Current time (respects virtual clock)
- `{{now+Xh}}` - X hours from now (e.g., `{{now+2h}}`)
- `{{now-Xm}}` - X minutes ago (e.g., `{{now-30m}}`)
- `{{now+Xd}}` - X days from now (e.g., `{{now+7d}}`)
- `{{now-Xs}}` - X seconds ago (e.g., `{{now-10s}}`)

Units: `s` (seconds), `m` (minutes), `h` (hours), `d` (days)

## Integration Testing

### Python Example

```python
import requests
import time

def test_token_expiry():
    admin_url = "http://localhost:9080"
    api_url = "http://localhost:3000"

    # Enable time travel
    requests.post(
        f"{admin_url}/__mockforge/time-travel/enable",
        json={"time": "2025-01-01T00:00:00Z"}
    )

    # Get token
    response = requests.get(f"{api_url}/auth/token")
    token = response.json()["token"]

    # Token should work
    response = requests.get(
        f"{api_url}/api/data",
        headers={"Authorization": f"Bearer {token}"}
    )
    assert response.status_code == 200

    # Advance past expiry
    requests.post(
        f"{admin_url}/__mockforge/time-travel/advance",
        json={"duration": "2h"}
    )

    # Token should be expired
    response = requests.get(
        f"{api_url}/api/data",
        headers={"Authorization": f"Bearer {token}"}
    )
    assert response.status_code == 401

    # Cleanup
    requests.post(f"{admin_url}/__mockforge/time-travel/disable")
```

### JavaScript Example

```javascript
const axios = require('axios');

async function testTokenExpiry() {
  const adminUrl = 'http://localhost:9080';
  const apiUrl = 'http://localhost:3000';

  // Enable time travel
  await axios.post(`${adminUrl}/__mockforge/time-travel/enable`, {
    time: '2025-01-01T00:00:00Z'
  });

  // Get token
  const tokenResponse = await axios.get(`${apiUrl}/auth/token`);
  const token = tokenResponse.data.token;

  // Token should work
  let response = await axios.get(`${apiUrl}/api/data`, {
    headers: { Authorization: `Bearer ${token}` }
  });
  console.assert(response.status === 200);

  // Advance past expiry
  await axios.post(`${adminUrl}/__mockforge/time-travel/advance`, {
    duration: '2h'
  });

  // Token should be expired
  try {
    response = await axios.get(`${apiUrl}/api/data`, {
      headers: { Authorization: `Bearer ${token}` }
    });
  } catch (error) {
    console.assert(error.response.status === 401);
  }

  // Cleanup
  await axios.post(`${adminUrl}/__mockforge/time-travel/disable`);
}
```

## Tips

1. **Always reset between tests** - Use the reset endpoint to ensure clean state
2. **Use relative times** - `+1h` is more flexible than absolute timestamps
3. **Monitor scheduled responses** - Use the list endpoint to see what's scheduled
4. **Combine with other features** - Time travel works with latency, failures, etc.
5. **Template expansion required** - Ensure `response_template_expand: true` in config

## Troubleshooting

**Time not advancing:**
- Check that time travel is enabled: `curl http://localhost:9080/__mockforge/time-travel/status`
- Verify advance request succeeded
- Check duration format (e.g., "2h", not "2 hours")

**Templates not working:**
- Ensure `response_template_expand: true` in config
- Verify time travel is enabled
- Check that templates use correct syntax: `{{now}}`

**Scheduled responses not triggering:**
- Verify trigger time hasn't passed
- Check that `enable_scheduling: true` in config
- Make sure you're making requests after advancing time
- List scheduled responses to verify they exist

## Documentation

For complete documentation, see [docs/TIME_TRAVEL.md](../docs/TIME_TRAVEL.md)

## Support

- GitHub Issues: https://github.com/SaaSy-Solutions/mockforge/issues
- Documentation: https://docs.mockforge.dev/

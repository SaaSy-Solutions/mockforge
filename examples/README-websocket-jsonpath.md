# WebSocket JSONPath Matching

MockForge now supports JSONPath queries for WebSocket message matching in addition to traditional regex patterns. This allows you to create more sophisticated WebSocket interactions based on the content of JSON messages.

## Overview

The WebSocket replay system now supports both regex and JSONPath patterns in the `waitFor` field of replay scripts. JSONPath patterns are automatically detected and used when the pattern starts with `$.` or `$[`.

## Pattern Detection

- **JSONPath patterns**: Start with `$.` or `$[` (e.g., `$.type`, `$[0].status`)
- **Regex patterns**: All other patterns (e.g., `^CLIENT_READY$`, `ACK`)

## JSONPath Examples

### Basic Property Matching

```json
{"waitFor": "$.type", "text": "Welcome to chat!"}
```

This waits for any JSON message that contains a `type` property, regardless of its value.

### Specific Value Matching

```json
{"waitFor": "$.status", "text": "Order processed"}
```

Waits for messages containing a `status` property.

### Nested Property Matching

```json
{"waitFor": "$.user.profile.name", "text": "Profile updated"}
```

Waits for messages with a nested structure: `{"user": {"profile": {"name": "..."}}}`

### Array Element Matching

```json
{"waitFor": "$.items[0].id", "text": "First item processed"}
```

Waits for messages with an array where the first element has an `id` property.

## Complete Example

```json
[
  {"ts": 0, "dir": "out", "text": "HELLO {{uuid}}", "waitFor": "^CLIENT_READY$"},
  {"ts": 10, "dir": "out", "text": "Welcome!", "waitFor": "$.type"},
  {"ts": 20, "dir": "out", "text": "User authenticated", "waitFor": "$.user.id"},
  {"ts": 30, "dir": "out", "text": "Order confirmed", "waitFor": "$.order.status"}
]
```

## JSONPath Query Language

JSONPath supports the following syntax:

- `$.property` - Access object property
- `$.property.subproperty` - Access nested property
- `$.array[index]` - Access array element by index
- `$.array[*]` - Access all array elements
- `$.array[?(@.property == 'value')]` - Filter array elements by condition

## Error Handling

- Invalid JSONPath queries will log a warning and return `false` (no match)
- Non-JSON messages will not match JSONPath queries (return `false`)
- Invalid regex patterns will log a warning and return `false`

## Performance Considerations

- JSONPath queries require parsing the WebSocket message as JSON
- Complex JSONPath queries may impact WebSocket performance
- Consider using simple property existence checks (`$.property`) for better performance

## Integration with Existing Features

- Works seamlessly with existing regex patterns
- Maintains backward compatibility with existing replay files
- Supports all WebSocket templating features (`{{uuid}}`, `{{now}}`, etc.)
- Compatible with latency injection and proxy features

## Testing

The updated `ws-demo.jsonl` file includes examples of both regex and JSONPath patterns:

```bash
# Run with JSONPath examples
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl
mockforge ws --port 3001
```

Then connect with a WebSocket client and send JSON messages that match the patterns.

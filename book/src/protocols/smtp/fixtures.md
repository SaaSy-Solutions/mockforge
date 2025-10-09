# SMTP Fixtures

SMTP fixtures allow you to define email acceptance rules, auto-reply behavior, and storage options based on pattern matching. This enables sophisticated email testing scenarios.

## Fixture Format

Fixtures are defined in YAML format:

```yaml
identifier: "welcome-email"
name: "Welcome Email Handler"
description: "Handles welcome emails to new users"

match_criteria:
  recipient_pattern: "^welcome@example\\.com$"
  sender_pattern: null
  subject_pattern: null
  match_all: false

response:
  status_code: 250
  message: "Message accepted"
  delay_ms: 0

auto_reply:
  enabled: false

storage:
  save_to_mailbox: true
  export_to_file: null

behavior:
  failure_rate: 0.0
  delay_ms: 0
```

## Match Criteria

### `recipient_pattern`

- **Type**: `string` (regex) or `null`
- **Description**: Regular expression to match recipient email address
- **Examples**:
  - `^user@example\.com$` - Exact match
  - `^.*@example\.com$` - Any user at domain
  - `^admin.*@.*\.com$` - Admin users at any .com domain

```yaml
match_criteria:
  recipient_pattern: "^support@example\\.com$"
```

### `sender_pattern`

- **Type**: `string` (regex) or `null`
- **Description**: Regular expression to match sender email address

```yaml
match_criteria:
  sender_pattern: "^no-reply@.*\\.com$"
```

### `subject_pattern`

- **Type**: `string` (regex) or `null`
- **Description**: Regular expression to match email subject line

```yaml
match_criteria:
  subject_pattern: "^\\[URGENT\\].*"
```

### `match_all`

- **Type**: `boolean`
- **Default**: `false`
- **Description**: When `true`, this fixture matches all emails (catch-all)

```yaml
match_criteria:
  match_all: true  # Catch-all fixture
```

### Matching Logic

Patterns are evaluated in order:

1. If `match_all` is `true`, fixture matches
2. Otherwise, **all non-null patterns must match**:
   - If `recipient_pattern` is set, it must match
   - If `sender_pattern` is set, it must match
   - If `subject_pattern` is set, it must match

## Response Configuration

### `status_code`

- **Type**: `integer`
- **Default**: `250`
- **Description**: SMTP status code to return
- **Common codes**:
  - `250` - OK (success)
  - `550` - Mailbox unavailable (rejection)
  - `451` - Temporary failure
  - `452` - Insufficient storage

```yaml
response:
  status_code: 550  # Reject email
```

### `message`

- **Type**: `string`
- **Description**: Response message text

```yaml
response:
  status_code: 250
  message: "Message accepted for delivery"
```

### `delay_ms`

- **Type**: `integer`
- **Default**: `0`
- **Description**: Artificial delay before responding (milliseconds)
- **Use case**: Simulate slow mail servers

```yaml
response:
  delay_ms: 500  # 500ms delay
```

## Auto-Reply

Auto-replies allow MockForge to automatically send response emails.

### Basic Auto-Reply

```yaml
auto_reply:
  enabled: true
  from: "noreply@example.com"
  to: "{{from}}"  # Reply to sender
  subject: "Re: {{subject}}"
  body: |
    Thank you for your email.

    This is an automated response.
```

### Template Variables

Use template variables in auto-reply fields:

- `{{from}}` - Original sender email
- `{{to}}` - Original recipient email
- `{{subject}}` - Original subject
- `{{from_name}}` - Extracted name from sender
- `{{now}}` - Current timestamp
- Faker functions: `{{faker.name}}`, `{{faker.email}}`, etc.

### Example: Welcome Email Auto-Reply

```yaml
identifier: "welcome-autoresponder"
name: "Welcome Email Auto-Reply"

match_criteria:
  recipient_pattern: "^register@example\\.com$"

response:
  status_code: 250
  message: "Message accepted"

auto_reply:
  enabled: true
  from: "welcome@example.com"
  to: "{{from}}"
  subject: "Welcome to Example.com!"
  body: |
    Hi {{from_name}},

    Thank you for registering at Example.com!

    Your registration was received at {{now}}.

    If you have any questions, reply to this email.

    Best regards,
    The Example.com Team
```

## Storage Configuration

### `save_to_mailbox`

- **Type**: `boolean`
- **Default**: `true`
- **Description**: Store received email in in-memory mailbox

```yaml
storage:
  save_to_mailbox: true
```

### `export_to_file`

- **Type**: `string` (path) or `null`
- **Description**: Export email to file on disk
- **Format**: Emails are saved as `.eml` files

```yaml
storage:
  save_to_mailbox: true
  export_to_file: "./emails/received"
```

File naming pattern: `{timestamp}_{from}_{to}.eml`

Example: `20240315_143022_sender@example.com_recipient@example.com.eml`

## Behavior Configuration

### `failure_rate`

- **Type**: `float` (0.0 to 1.0)
- **Default**: `0.0`
- **Description**: Probability of simulated failure (for testing error handling)
- **Examples**:
  - `0.0` - Never fail
  - `0.1` - 10% failure rate
  - `1.0` - Always fail

```yaml
behavior:
  failure_rate: 0.05  # 5% of emails fail
```

### `delay_ms`

- **Type**: `integer`
- **Default**: `0`
- **Description**: Artificial delay before processing (milliseconds)

```yaml
behavior:
  delay_ms: 1000  # 1 second delay
```

## Complete Examples

### Example 1: User Registration Emails

```yaml
identifier: "user-registration"
name: "User Registration Handler"
description: "Handles new user registration confirmation emails"

match_criteria:
  recipient_pattern: "^[^@]+@example\\.com$"
  subject_pattern: "^Registration Confirmation"

response:
  status_code: 250
  message: "Registration email accepted"
  delay_ms: 0

auto_reply:
  enabled: true
  from: "noreply@example.com"
  to: "{{from}}"
  subject: "Welcome! Please Confirm Your Email"
  body: |
    Hello,

    Thank you for registering!

    Please click the link below to confirm your email:
    https://example.com/confirm?token={{uuid}}

    This link expires in 24 hours.

    Best regards,
    Example.com Team

storage:
  save_to_mailbox: true
  export_to_file: "./logs/registration-emails"

behavior:
  failure_rate: 0.0
  delay_ms: 0
```

### Example 2: Support Ticket System

```yaml
identifier: "support-tickets"
name: "Support Ticket Handler"
description: "Auto-responds to support emails"

match_criteria:
  recipient_pattern: "^support@example\\.com$"

response:
  status_code: 250
  message: "Support ticket created"

auto_reply:
  enabled: true
  from: "support@example.com"
  to: "{{from}}"
  subject: "Ticket Created: {{subject}}"
  body: |
    Your support ticket has been created.

    Ticket ID: {{uuid}}
    Subject: {{subject}}
    Created: {{now}}

    We'll respond within 24 hours.

    Support Team

storage:
  save_to_mailbox: true
```

### Example 3: Bounced Email Simulation

```yaml
identifier: "bounce-simulation"
name: "Simulate Bounced Emails"
description: "Rejects emails to invalid addresses"

match_criteria:
  recipient_pattern: "^bounce-test@example\\.com$"

response:
  status_code: 550
  message: "Mailbox unavailable"
  delay_ms: 0

auto_reply:
  enabled: false

storage:
  save_to_mailbox: false

behavior:
  failure_rate: 1.0  # Always fail
```

### Example 4: Slow Server Simulation

```yaml
identifier: "slow-server"
name: "Slow SMTP Server"
description: "Simulates slow mail server response"

match_criteria:
  recipient_pattern: "^slowtest@example\\.com$"

response:
  status_code: 250
  message: "OK"
  delay_ms: 5000  # 5 second delay

storage:
  save_to_mailbox: true

behavior:
  delay_ms: 3000  # Additional 3 second processing delay
```

### Example 5: Catch-All Default

```yaml
identifier: "default-handler"
name: "Default Email Handler"
description: "Accepts all emails not matched by other fixtures"

match_criteria:
  match_all: true

response:
  status_code: 250
  message: "Message accepted"

auto_reply:
  enabled: false

storage:
  save_to_mailbox: true

behavior:
  failure_rate: 0.0
  delay_ms: 0
```

## Loading Fixtures

### Directory Structure

```
fixtures/smtp/
├── welcome-email.yaml
├── support-tickets.yaml
├── bounce-simulation.yaml
└── default.yaml
```

### Configuration

```yaml
smtp:
  fixtures_dir: "./fixtures/smtp"
```

### Fixture Priority

Fixtures are evaluated in **alphabetical order by filename**. First match wins (except `match_all`).

To control priority, use numbered prefixes:

```
fixtures/smtp/
├── 01-bounce.yaml       # Highest priority
├── 02-welcome.yaml
├── 03-support.yaml
└── 99-default.yaml      # Lowest priority (catch-all)
```

## Testing Fixtures

### 1. Validate Fixture Syntax

```bash
# Future command (not yet implemented)
mockforge smtp fixtures validate ./fixtures/smtp/welcome.yaml
```

### 2. Test Fixture Matching

Send test email:

```bash
swaks --to welcome@example.com \
      --from test@test.com \
      --server localhost:1025 \
      --header "Subject: Test"
```

Check server logs for fixture match:
```
[INFO] Matched fixture: welcome-email
```

### 3. Verify Auto-Reply

Check mailbox or export directory for auto-reply email.

## Best Practices

### 1. Specific Before General

Place specific fixtures before general catch-all fixtures:

```
01-specific-user.yaml
02-domain-specific.yaml
99-catch-all.yaml
```

### 2. Use Descriptive Identifiers

```yaml
identifier: "welcome-new-users"  # Good
identifier: "fixture1"            # Bad
```

### 3. Document with Descriptions

```yaml
description: "Handles password reset emails with confirmation link"
```

### 4. Test Failure Scenarios

```yaml
behavior:
  failure_rate: 0.01  # Test with 1% failure
```

### 5. Limit Auto-Replies

Don't create auto-reply loops:
- Avoid auto-replying to `noreply@` addresses
- Check sender before replying

## Troubleshooting

### Fixture Not Matching

1. **Check pattern syntax**: Use regex tester (regex101.com)
2. **Check fixture order**: Earlier fixtures may match first
3. **Enable debug logging**: See which fixture matched
4. **Test with simple pattern**: Start with `^.*@example\.com$`

### Auto-Reply Not Sending

1. **Verify enabled**: `auto_reply.enabled: true`
2. **Check template syntax**: Ensure valid template variables
3. **Check logs**: Look for auto-reply errors

### Performance Issues

1. **Simplify regex**: Complex patterns slow matching
2. **Reduce fixtures**: Too many fixtures slow evaluation
3. **Disable storage**: Set `save_to_mailbox: false` if not needed

## Related Documentation

- [Getting Started](./getting-started.md)
- [Configuration](./configuration.md)
- [Examples](./examples.md)

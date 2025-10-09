# Getting Started with SMTP

MockForge includes a fully functional SMTP (Simple Mail Transfer Protocol) server for testing email workflows in your applications. This guide will help you get started quickly.

## Quick Start

### 1. Enable SMTP in Configuration

Create a configuration file or modify your existing `config.yaml`:

```yaml
smtp:
  enabled: true
  port: 1025
  host: "0.0.0.0"
  hostname: "mockforge-smtp"
```

### 2. Start the Server

```bash
mockforge serve --config config.yaml
```

You should see:
```
📧 SMTP server listening on localhost:1025
```

### 3. Send a Test Email

Using Python's built-in `smtplib`:

```python
import smtplib
from email.message import EmailMessage

msg = EmailMessage()
msg['Subject'] = 'Test Email'
msg['From'] = 'sender@example.com'
msg['To'] = 'recipient@example.com'
msg.set_content('This is a test email from Python.')

with smtplib.SMTP('localhost', 1025) as server:
    server.send_message(msg)
    print("Email sent successfully!")
```

### 4. Verify Email Reception

Currently, emails are stored in the in-memory mailbox. You can verify by checking the server logs or using the API endpoints (if UI is enabled).

## Using Command-Line Tools

### telnet

```bash
telnet localhost 1025
> EHLO client.example.com
> MAIL FROM:<sender@example.com>
> RCPT TO:<recipient@example.com>
> DATA
> Subject: Test Email
>
> This is a test email.
> .
> QUIT
```

### swaks (SMTP Testing Tool)

[swaks](http://www.jetmore.org/john/code/swaks/) is a powerful SMTP testing tool:

```bash
# Install swaks
# On Ubuntu/Debian: apt install swaks
# On macOS: brew install swaks

# Send test email
swaks --to recipient@example.com \
      --from sender@example.com \
      --server localhost:1025 \
      --body "Test email from swaks" \
      --header "Subject: Test"
```

## Supported SMTP Commands

MockForge SMTP server implements RFC 5321 and supports:

- **HELO** / **EHLO** - Client introduction
- **MAIL FROM** - Specify sender
- **RCPT TO** - Specify recipient(s)
- **DATA** - Send message content
- **RSET** - Reset session
- **NOOP** - No operation (keepalive)
- **QUIT** - End session
- **HELP** - List supported commands

## Basic Configuration Options

```yaml
smtp:
  enabled: true               # Enable/disable SMTP server
  port: 1025                  # Port (1025 for dev, 25 for prod)
  host: "0.0.0.0"             # Bind address
  hostname: "mockforge-smtp"  # Server hostname in greeting

  # Mailbox settings
  enable_mailbox: true
  max_mailbox_messages: 1000

  # Timeouts
  timeout_secs: 30
  max_connections: 100
```

## Environment Variables

Override configuration with environment variables:

```bash
export MOCKFORGE_SMTP_ENABLED=true
export MOCKFORGE_SMTP_PORT=1025
export MOCKFORGE_SMTP_HOST=0.0.0.0
export MOCKFORGE_SMTP_HOSTNAME=my-smtp-server

mockforge serve
```

## Next Steps

- [Configuration Reference](./configuration.md) - Detailed configuration options
- [Fixtures](./fixtures.md) - Create email scenarios and auto-replies
- [Examples](./examples.md) - Real-world usage examples

## Troubleshooting

### Connection Refused

**Problem**: Cannot connect to SMTP server

**Solutions**:
1. Verify SMTP is enabled: `smtp.enabled: true`
2. Check the port isn't in use: `lsof -i :1025`
3. Ensure server is running: Look for "SMTP server listening" in logs

### Email Not Received

**Problem**: Email sent but not stored

**Solutions**:
1. Check mailbox is enabled: `smtp.enable_mailbox: true`
2. Verify mailbox size limit: `smtp.max_mailbox_messages`
3. Check server logs for errors

### Permission Denied on Port 25

**Problem**: Cannot bind to port 25

**Solution**: Ports below 1024 require root privileges. Use port 1025 for development or run with sudo for production testing.

## Common Use Cases

### Testing Email Workflows

```python
# In your test suite
def test_user_registration_sends_welcome_email():
    # Register user (triggers email send)
    response = client.post('/register', json={
        'email': 'newuser@example.com',
        'password': 'secret'
    })

    assert response.status_code == 201

    # Verify email was sent to MockForge SMTP
    emails = get_emails_from_mockforge()
    assert len(emails) == 1
    assert emails[0]['to'] == 'newuser@example.com'
    assert 'Welcome' in emails[0]['subject']
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
- name: Start MockForge SMTP
  run: |
    mockforge serve --smtp --smtp-port 1025 &
    sleep 2

- name: Run tests
  env:
    SMTP_HOST: localhost
    SMTP_PORT: 1025
  run: pytest tests/
```

## What's Next?

Now that you have a basic SMTP server running, explore:

1. **[Fixtures](./fixtures.md)** - Define email acceptance rules and auto-replies
2. **[Configuration](./configuration.md)** - Fine-tune server behavior
3. **[Examples](./examples.md)** - See real-world implementations

# SMTP Examples

This page provides real-world examples of using MockForge SMTP for testing email workflows.

## Table of Contents

- [Testing User Registration](#testing-user-registration)
- [Password Reset Flow](#password-reset-flow)
- [Email Verification](#email-verification)
- [Newsletter Subscriptions](#newsletter-subscriptions)
- [CI/CD Integration](#cicd-integration)
- [Load Testing](#load-testing)
- [Multi-Language Applications](#multi-language-applications)

## Testing User Registration

### Scenario

Test that your application sends a welcome email when users register.

### Fixture

`fixtures/smtp/welcome-email.yaml`:

```yaml
identifier: "welcome-email"
name: "Welcome Email"
description: "Auto-responds to new user registration"

match_criteria:
  recipient_pattern: "^[^@]+@example\\.com$"
  subject_pattern: "^Welcome"

response:
  status_code: 250
  message: "Message accepted"

auto_reply:
  enabled: true
  from: "noreply@example.com"
  to: "{{from}}"
  subject: "Welcome to Our Platform!"
  body: |
    Hi there!

    Thank you for registering at our platform.

    Click here to verify your email:
    https://example.com/verify?token={{uuid}}

    Best regards,
    The Team

storage:
  save_to_mailbox: true
```

### Python Test

```python
import smtplib
import requests
from email.message import EmailMessage

def test_user_registration_sends_welcome_email():
    # Register a new user
    response = requests.post('http://localhost:8080/api/register', json={
        'email': 'newuser@example.com',
        'password': 'SecurePass123',
        'name': 'Test User'
    })

    assert response.status_code == 201

    # Verify email was sent
    # (In real scenario, you'd query MockForge's mailbox API)
    # For now, manually check logs or implement mailbox checking

def send_test_email():
    """Helper to test fixture directly"""
    msg = EmailMessage()
    msg['Subject'] = 'Welcome to Our Platform'
    msg['From'] = 'system@myapp.com'
    msg['To'] = 'newuser@example.com'
    msg.set_content('Welcome!')

    with smtplib.SMTP('localhost', 1025) as server:
        server.send_message(msg)
        print("Test email sent!")

if __name__ == "__main__":
    send_test_email()
```

### Node.js Test

```javascript
const nodemailer = require('nodemailer');
const axios = require('axios');
const assert = require('assert');

describe('User Registration', () => {
  it('should send welcome email', async () => {
    // Configure nodemailer to use MockForge
    const transporter = nodemailer.createTransport({
      host: 'localhost',
      port: 1025,
      secure: false,
    });

    // Register user
    const response = await axios.post('http://localhost:8080/api/register', {
      email: 'newuser@example.com',
      password: 'SecurePass123',
      name: 'Test User'
    });

    assert.strictEqual(response.status, 201);

    // Send test email
    await transporter.sendMail({
      from: 'system@myapp.com',
      to: 'newuser@example.com',
      subject: 'Welcome to Our Platform',
      text: 'Welcome!',
    });

    // In production, query MockForge mailbox API here
  });
});
```

## Password Reset Flow

### Scenario

Test password reset email with temporary token.

### Fixture

`fixtures/smtp/password-reset.yaml`:

```yaml
identifier: "password-reset"
name: "Password Reset"

match_criteria:
  recipient_pattern: "^.*@.*$"
  subject_pattern: "^Password Reset"

response:
  status_code: 250
  message: "Reset email accepted"

auto_reply:
  enabled: true
  from: "security@example.com"
  to: "{{from}}"
  subject: "Password Reset Instructions"
  body: |
    Hello,

    You requested a password reset.

    Click the link below to reset your password:
    https://example.com/reset?token={{uuid}}

    This link expires in 1 hour.

    If you didn't request this, please ignore this email.

    Security Team

storage:
  save_to_mailbox: true
  export_to_file: "./logs/password-resets"
```

### Python Test

```python
import pytest
import smtplib
from email.message import EmailMessage

def trigger_password_reset(email):
    """Trigger password reset in your application"""
    import requests
    response = requests.post('http://localhost:8080/api/password-reset',
                            json={'email': email})
    return response.status_code == 200

def test_password_reset_email():
    email = 'user@example.com'

    # Trigger reset
    assert trigger_password_reset(email)

    # Verify email sent (check mailbox)
    # TODO: Implement mailbox API check

def test_password_reset_invalid_email():
    """Test that invalid email is rejected"""
    email = 'bounce-test@example.com'  # Configured to fail

    # This should fail
    assert not trigger_password_reset(email)
```

## Email Verification

### Scenario

Test email verification link generation and sending.

### Fixture

`fixtures/smtp/email-verification.yaml`:

```yaml
identifier: "email-verification"
name: "Email Verification"

match_criteria:
  subject_pattern: "^Verify Your Email"

response:
  status_code: 250
  message: "Verification email sent"

auto_reply:
  enabled: true
  from: "noreply@example.com"
  to: "{{from}}"
  subject: "Verify Your Email Address"
  body: |
    Please verify your email address by clicking below:

    https://example.com/verify?email={{to}}&code={{faker.alphanumeric 32}}

    This link expires in 24 hours.

storage:
  save_to_mailbox: true
```

### Go Test

```go
package main

import (
    "net/smtp"
    "testing"
)

func TestEmailVerification(t *testing.T) {
    // Setup
    smtpHost := "localhost:1025"
    from := "system@myapp.com"
    to := []string{"user@example.com"}

    // Create message
    message := []byte(
        "Subject: Verify Your Email\r\n" +
        "\r\n" +
        "Please verify your email.\r\n",
    )

    // Send email
    err := smtp.SendMail(smtpHost, nil, from, to, message)
    if err != nil {
        t.Fatalf("Failed to send email: %v", err)
    }

    // Verify sent (check mailbox)
    // TODO: Implement mailbox check
}
```

## Newsletter Subscriptions

### Scenario

Test newsletter subscription confirmation emails.

### Fixture

`fixtures/smtp/newsletter.yaml`:

```yaml
identifier: "newsletter-subscription"
name: "Newsletter Subscription"

match_criteria:
  recipient_pattern: "^newsletter@example\\.com$"

response:
  status_code: 250
  message: "Subscription received"

auto_reply:
  enabled: true
  from: "newsletter@example.com"
  to: "{{from}}"
  subject: "Confirm Your Newsletter Subscription"
  body: |
    Thanks for subscribing to our newsletter!

    Click to confirm: https://example.com/newsletter/confirm?email={{from}}

    You'll receive our weekly digest every Monday.

storage:
  save_to_mailbox: true
```

### Ruby Test

```ruby
require 'mail'
require 'minitest/autorun'

class NewsletterTest < Minitest::Test
  def setup
    Mail.defaults do
      delivery_method :smtp,
        address: "localhost",
        port: 1025
    end
  end

  def test_newsletter_subscription
    email = Mail.new do
      from     'user@test.com'
      to       'newsletter@example.com'
      subject  'Subscribe'
      body     'Please subscribe me'
    end

    email.deliver!

    # Verify subscription email sent
    # TODO: Check MockForge mailbox
  end
end
```

## CI/CD Integration

### GitHub Actions

`.github/workflows/test.yml`:

```yaml
name: Test Email Workflows

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      mockforge:
        image: mockforge/mockforge:latest
        ports:
          - 1025:1025
        env:
          MOCKFORGE_SMTP_ENABLED: true
          MOCKFORGE_SMTP_PORT: 1025

    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          pip install -r requirements.txt

      - name: Run email tests
        env:
          SMTP_HOST: localhost
          SMTP_PORT: 1025
        run: |
          pytest tests/test_emails.py -v
```

### GitLab CI

`.gitlab-ci.yml`:

```yaml
test:
  image: python:3.11
  services:
    - name: mockforge/mockforge:latest
      alias: mockforge
  variables:
    MOCKFORGE_SMTP_ENABLED: "true"
    SMTP_HOST: mockforge
    SMTP_PORT: "1025"
  script:
    - pip install -r requirements.txt
    - pytest tests/test_emails.py
```

### Docker Compose

`docker-compose.test.yml`:

```yaml
version: '3.8'

services:
  mockforge:
    image: mockforge/mockforge:latest
    ports:
      - "1025:1025"
    environment:
      MOCKFORGE_SMTP_ENABLED: "true"
      MOCKFORGE_SMTP_PORT: 1025
    volumes:
      - ./fixtures:/fixtures

  app:
    build: .
    depends_on:
      - mockforge
    environment:
      SMTP_HOST: mockforge
      SMTP_PORT: 1025
    command: pytest tests/
```

## Load Testing

### Scenario

Test application performance with high email volume.

### Python Load Test

```python
import concurrent.futures
import smtplib
from email.message import EmailMessage
import time

def send_email(index):
    """Send a single email"""
    msg = EmailMessage()
    msg['Subject'] = f'Load Test Email {index}'
    msg['From'] = f'loadtest{index}@test.com'
    msg['To'] = 'recipient@example.com'
    msg.set_content(f'This is load test email #{index}')

    try:
        with smtplib.SMTP('localhost', 1025, timeout=5) as server:
            server.send_message(msg)
        return True
    except Exception as e:
        print(f"Error sending email {index}: {e}")
        return False

def load_test(num_emails=1000, num_workers=10):
    """Send many emails concurrently"""
    print(f"Starting load test: {num_emails} emails with {num_workers} workers")

    start_time = time.time()

    with concurrent.futures.ThreadPoolExecutor(max_workers=num_workers) as executor:
        results = list(executor.map(send_email, range(num_emails)))

    end_time = time.time()
    duration = end_time - start_time

    success_count = sum(results)
    emails_per_second = num_emails / duration

    print(f"\nResults:")
    print(f"  Total emails: {num_emails}")
    print(f"  Successful: {success_count}")
    print(f"  Failed: {num_emails - success_count}")
    print(f"  Duration: {duration:.2f}s")
    print(f"  Throughput: {emails_per_second:.2f} emails/sec")

if __name__ == "__main__":
    load_test(num_emails=1000, num_workers=20)
```

### Configuration for Load Testing

```yaml
smtp:
  enabled: true
  port: 1025
  host: "0.0.0.0"
  max_connections: 500
  max_mailbox_messages: 10000
  timeout_secs: 60
```

## Multi-Language Applications

### Scenario

Test internationalized email content.

### Fixture with Template

`fixtures/smtp/i18n-welcome.yaml`:

```yaml
identifier: "i18n-welcome"
name: "Internationalized Welcome"

match_criteria:
  recipient_pattern: "^[^@]+@example\\.com$"
  subject_pattern: "^Welcome|Bienvenue|Willkommen"

response:
  status_code: 250
  message: "Message accepted"

auto_reply:
  enabled: false  # Handle in application

storage:
  save_to_mailbox: true
```

### Python Multi-Language Test

```python
import smtplib
from email.message import EmailMessage
from email.mime.text import MIMEText

def send_welcome_email(recipient, language='en'):
    """Send welcome email in specified language"""

    subjects = {
        'en': 'Welcome to Our Platform',
        'fr': 'Bienvenue sur notre plateforme',
        'de': 'Willkommen auf unserer Plattform',
        'es': 'Bienvenido a nuestra plataforma'
    }

    bodies = {
        'en': 'Welcome! Thank you for registering.',
        'fr': 'Bienvenue! Merci de vous être inscrit.',
        'de': 'Willkommen! Danke für Ihre Registrierung.',
        'es': '¡Bienvenido! Gracias por registrarse.'
    }

    msg = EmailMessage()
    msg['Subject'] = subjects.get(language, subjects['en'])
    msg['From'] = 'noreply@example.com'
    msg['To'] = recipient
    msg['Content-Language'] = language
    msg.set_content(bodies.get(language, bodies['en']))

    with smtplib.SMTP('localhost', 1025) as server:
        server.send_message(msg)

def test_multi_language_emails():
    """Test emails in multiple languages"""
    languages = ['en', 'fr', 'de', 'es']

    for lang in languages:
        send_welcome_email(f'user-{lang}@example.com', lang)
        print(f"Sent {lang} email")

if __name__ == "__main__":
    test_multi_language_emails()
```

## Testing Email Bounces

### Scenario

Test application handling of bounced emails.

### Fixture

`fixtures/smtp/bounce-test.yaml`:

```yaml
identifier: "bounce-simulation"
name: "Bounce Simulation"

match_criteria:
  recipient_pattern: "^bounce@example\\.com$"

response:
  status_code: 550
  message: "Mailbox unavailable"

storage:
  save_to_mailbox: false

behavior:
  failure_rate: 1.0  # Always fail
```

### Test

```python
import smtplib
from email.message import EmailMessage

def test_bounce_handling():
    """Test that application handles bounces correctly"""

    msg = EmailMessage()
    msg['Subject'] = 'Test Bounce'
    msg['From'] = 'sender@test.com'
    msg['To'] = 'bounce@example.com'
    msg.set_content('This should bounce')

    try:
        with smtplib.SMTP('localhost', 1025) as server:
            server.send_message(msg)
        assert False, "Expected SMTPRecipientsRefused"
    except smtplib.SMTPRecipientsRefused as e:
        # Expected behavior
        print(f"Bounce handled correctly: {e}")
        assert '550' in str(e)
```

## Integration with Testing Frameworks

### pytest Fixture

```python
import pytest
import smtplib
from email.message import EmailMessage

@pytest.fixture
def smtp_client():
    """Provides SMTP client connected to MockForge"""
    return smtplib.SMTP('localhost', 1025)

@pytest.fixture
def email_factory():
    """Factory for creating test emails"""
    def _create_email(to, subject="Test", body="Test body"):
        msg = EmailMessage()
        msg['Subject'] = subject
        msg['From'] = 'test@test.com'
        msg['To'] = to
        msg.set_content(body)
        return msg
    return _create_email

def test_with_fixtures(smtp_client, email_factory):
    """Test using pytest fixtures"""
    email = email_factory('user@example.com', subject='Welcome')
    smtp_client.send_message(email)
    # Verify email sent
```

### unittest Helper

```python
import unittest
import smtplib

class EmailTestCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        """Set up SMTP connection for all tests"""
        cls.smtp_host = 'localhost'
        cls.smtp_port = 1025

    def send_test_email(self, to, subject, body):
        """Helper method to send test emails"""
        with smtplib.SMTP(self.smtp_host, self.smtp_port) as server:
            # ... send email
            pass

    def test_email_sending(self):
        self.send_test_email('test@example.com', 'Test', 'Body')
        # Verify
```

## Best Practices

1. **Use dedicated fixtures** for each test scenario
2. **Clean mailbox** between test runs
3. **Test both success and failure** scenarios
4. **Verify email content**, not just delivery
5. **Use realistic delays** in load tests
6. **Test internationalization** early
7. **Mock external dependencies** completely

## Troubleshooting

### Emails Not Received

Check:
1. SMTP server is running
2. Correct port (1025)
3. Fixture patterns match
4. Mailbox not full

### Slow Tests

Optimize:
1. Reduce `delay_ms` in fixtures
2. Disable `save_to_mailbox` if not needed
3. Use concurrent connections in load tests

### Fixture Not Matching

Debug:
1. Enable debug logging
2. Simplify regex patterns
3. Test patterns with regex101.com
4. Check fixture load order

## Related Documentation

- [Getting Started](./getting-started.md)
- [Configuration](./configuration.md)
- [Fixtures](./fixtures.md)

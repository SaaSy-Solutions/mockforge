# mockforge-smtp

SMTP server mocking for MockForge.

## Features

- RFC 5321 compliant SMTP server
- Fixture-based email handling
- Auto-reply configuration
- In-memory mailbox storage
- Template-based email generation
- Integration with MockForge protocol abstraction

## Quick Start

```rust
use mockforge_smtp::{SmtpServer, SmtpConfig, SmtpSpecRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SmtpConfig::default();
    let registry = Arc::new(SmtpSpecRegistry::new());

    let server = SmtpServer::new(config, registry);
    server.start().await?;

    Ok(())
}
```

## Configuration

```yaml
smtp:
  enabled: true
  port: 1025
  host: "0.0.0.0"
  hostname: "mockforge-smtp"
  fixtures_dir: "./fixtures/smtp"
```

## License

MIT OR Apache-2.0

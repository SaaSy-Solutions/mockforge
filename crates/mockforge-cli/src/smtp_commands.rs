//! SMTP server management and mailbox operations

use clap::Subcommand;
use mockforge_smtp::SmtpFixture;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum SmtpCommands {
    /// Mailbox management commands
    Mailbox {
        #[command(subcommand)]
        mailbox_command: MailboxCommands,
    },

    /// Fixture management commands
    Fixtures {
        #[command(subcommand)]
        fixtures_command: FixturesCommands,
    },

    /// Send test email
    Send {
        /// Recipient email address
        #[arg(short, long)]
        to: String,

        /// Email subject
        #[arg(short, long)]
        subject: String,

        /// Email body
        #[arg(short, long, default_value = "Test email from MockForge CLI")]
        body: String,

        /// SMTP server host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// SMTP server port
        #[arg(long, default_value = "1025")]
        port: u16,

        /// Sender email address
        #[arg(long, default_value = "test@mockforge.cli")]
        from: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum MailboxCommands {
    /// List all emails in mailbox
    List,

    /// Show details of specific email
    Show {
        /// Email ID
        email_id: String,
    },

    /// Clear all emails from mailbox
    Clear,

    /// Export mailbox to file
    Export {
        /// Output format (mbox, json, csv)
        #[arg(short, long, default_value = "mbox")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Search emails in mailbox
    Search {
        /// Filter by sender email
        #[arg(long)]
        sender: Option<String>,

        /// Filter by recipient email
        #[arg(long)]
        recipient: Option<String>,

        /// Filter by subject
        #[arg(long)]
        subject: Option<String>,

        /// Filter by body content
        #[arg(long)]
        body: Option<String>,

        /// Filter emails since date (RFC3339 format)
        #[arg(long)]
        since: Option<String>,

        /// Filter emails until date (RFC3339 format)
        #[arg(long)]
        until: Option<String>,

        /// Use regex matching instead of substring
        #[arg(long)]
        regex: bool,

        /// Case sensitive matching
        #[arg(long)]
        case_sensitive: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum FixturesCommands {
    /// List loaded fixtures
    List,

    /// Reload fixtures from disk
    Reload,

    /// Validate fixture file
    Validate {
        /// Fixture file path
        file: PathBuf,
    },
}

/// Handle SMTP commands
pub async fn handle_smtp_command(
    smtp_command: SmtpCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match smtp_command {
        SmtpCommands::Mailbox { mailbox_command } => {
            handle_mailbox_command(mailbox_command).await?;
        }
        SmtpCommands::Fixtures { fixtures_command } => {
            handle_fixtures_command(fixtures_command).await?;
        }
        SmtpCommands::Send {
            to,
            subject,
            body,
            host,
            port,
            from,
        } => {
            handle_send_command(to, subject, body, host, port, from).await?;
        }
    }
    Ok(())
}

/// Handle mailbox management commands
async fn handle_mailbox_command(
    mailbox_command: MailboxCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match mailbox_command {
        MailboxCommands::List => {
            handle_mailbox_list().await?;
        }
        MailboxCommands::Show { email_id } => {
            handle_mailbox_show(&email_id).await?;
        }
        MailboxCommands::Clear => {
            handle_mailbox_clear().await?;
        }
        MailboxCommands::Export { format, output } => {
            handle_mailbox_export(&format, &output).await?;
        }
        MailboxCommands::Search {
            sender,
            recipient,
            subject,
            body,
            since,
            until,
            regex,
            case_sensitive,
        } => {
            handle_mailbox_search(
                sender,
                recipient,
                subject,
                body,
                since,
                until,
                regex,
                case_sensitive,
            )
            .await?;
        }
    }
    Ok(())
}

/// List emails in mailbox
async fn handle_mailbox_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📧 Listing emails in mailbox...");

    // Try to connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.get(format!("{}/smtp/mailbox", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let emails: Vec<serde_json::Value> = response.json().await?;
                if emails.is_empty() {
                    println!("📭 Mailbox is empty");
                } else {
                    println!("📬 Found {} emails:", emails.len());
                    println!("{:<5} {:<30} {:<50} Received", "ID", "From", "Subject");
                    println!("{}", "-".repeat(100));

                    for email in emails {
                        let id = email["id"].as_str().unwrap_or("N/A");
                        let from = email["from"].as_str().unwrap_or("N/A");
                        let subject = email["subject"].as_str().unwrap_or("N/A");
                        let received = email["received_at"].as_str().unwrap_or("N/A");

                        // Truncate subject if too long
                        let subject_display = if subject.len() > 47 {
                            format!("{}...", &subject[..44])
                        } else {
                            subject.to_string()
                        };

                        println!(
                            "{:<5} {:<30} {:<50} {}",
                            &id[..std::cmp::min(id.len(), 5)],
                            &from[..std::cmp::min(from.len(), 30)],
                            subject_display,
                            received
                        );
                    }
                }
            } else {
                println!("❌ Failed to access mailbox: HTTP {}", response.status());
                println!("💡 Make sure MockForge server is running with SMTP enabled");
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running at {}", management_url);
            println!("💡 Or set MOCKFORGE_MANAGEMENT_URL environment variable");
        }
    }

    Ok(())
}

/// Show email details
async fn handle_mailbox_show(
    email_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📧 Showing email {}...", email_id);

    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.get(format!("{}/smtp/mailbox/{}", management_url, email_id)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let email: serde_json::Value = response.json().await?;
                println!("📧 Email Details:");
                println!("ID: {}", email["id"].as_str().unwrap_or("N/A"));
                println!("From: {}", email["from"].as_str().unwrap_or("N/A"));
                println!(
                    "To: {}",
                    email["to"]
                        .as_array()
                        .map(|to| to
                            .iter()
                            .map(|t| t.as_str().unwrap_or("N/A"))
                            .collect::<Vec<_>>()
                            .join(", "))
                        .unwrap_or_else(|| "N/A".to_string())
                );
                println!("Subject: {}", email["subject"].as_str().unwrap_or("N/A"));
                println!("Received: {}", email["received_at"].as_str().unwrap_or("N/A"));
                println!();
                println!("Headers:");
                if let Some(headers) = email["headers"].as_object() {
                    for (key, value) in headers {
                        println!("  {}: {}", key, value.as_str().unwrap_or("N/A"));
                    }
                }
                println!();
                println!("Body:");
                println!("{}", email["body"].as_str().unwrap_or("N/A"));
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ Email with ID '{}' not found", email_id);
            } else {
                println!("❌ Failed to retrieve email: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running");
        }
    }

    Ok(())
}

/// Clear mailbox
async fn handle_mailbox_clear() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🗑️  Clearing mailbox...");

    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.delete(format!("{}/smtp/mailbox", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Mailbox cleared successfully");
            } else {
                println!("❌ Failed to clear mailbox: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running");
        }
    }

    Ok(())
}

/// Export mailbox
async fn handle_mailbox_export(
    format: &str,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Exporting mailbox to {} in {} format...", output.display(), format);

    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client
        .get(format!("{}/smtp/mailbox/export?format={}", management_url, format))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                let content = response.text().await?;
                std::fs::write(output, content)?;
                println!("✅ Mailbox exported to {}", output.display());
            } else {
                println!("❌ Failed to export mailbox: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running");
        }
    }

    Ok(())
}

/// Search emails in mailbox
#[allow(clippy::too_many_arguments)]
async fn handle_mailbox_search(
    sender: Option<String>,
    recipient: Option<String>,
    subject: Option<String>,
    body: Option<String>,
    since: Option<String>,
    until: Option<String>,
    regex: bool,
    case_sensitive: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Searching emails in mailbox...");

    let mut query_params = Vec::new();
    if let Some(ref s) = sender {
        query_params.push(format!("sender={}", urlencoding::encode(s)));
    }
    if let Some(ref r) = recipient {
        query_params.push(format!("recipient={}", urlencoding::encode(r)));
    }
    if let Some(ref s) = subject {
        query_params.push(format!("subject={}", urlencoding::encode(s)));
    }
    if let Some(ref b) = body {
        query_params.push(format!("body={}", urlencoding::encode(b)));
    }
    if let Some(ref s) = since {
        query_params.push(format!("since={}", urlencoding::encode(s)));
    }
    if let Some(ref u) = until {
        query_params.push(format!("until={}", urlencoding::encode(u)));
    }
    if regex {
        query_params.push("regex=true".to_string());
    }
    if case_sensitive {
        query_params.push("case_sensitive=true".to_string());
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client
        .get(format!("{}/smtp/mailbox/search{}", management_url, query_string))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                let emails: Vec<serde_json::Value> = response.json().await?;
                if emails.is_empty() {
                    println!("🔍 No emails found matching the criteria");
                } else {
                    println!("🔍 Found {} emails:", emails.len());
                    println!("{:<5} {:<30} {:<50} Received", "ID", "From", "Subject");
                    println!("{}", "-".repeat(100));

                    for email in emails {
                        let id = email["id"].as_str().unwrap_or("N/A");
                        let from = email["from"].as_str().unwrap_or("N/A");
                        let subject = email["subject"].as_str().unwrap_or("N/A");
                        let received = email["received_at"].as_str().unwrap_or("N/A");

                        let subject_display = if subject.len() > 47 {
                            format!("{}...", &subject[..44])
                        } else {
                            subject.to_string()
                        };

                        println!(
                            "{:<5} {:<30} {:<50} {}",
                            &id[..std::cmp::min(id.len(), 5)],
                            &from[..std::cmp::min(from.len(), 30)],
                            subject_display,
                            received
                        );
                    }
                }
            } else {
                println!("❌ Failed to search mailbox: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running");
        }
    }

    Ok(())
}

/// Handle fixture management commands
async fn handle_fixtures_command(
    fixtures_command: FixturesCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match fixtures_command {
        FixturesCommands::List => {
            handle_fixtures_list().await?;
        }
        FixturesCommands::Reload => {
            handle_fixtures_reload().await?;
        }
        FixturesCommands::Validate { file } => {
            handle_fixtures_validate(&file).await?;
        }
    }
    Ok(())
}

/// Handle send test email command
async fn handle_send_command(
    to: String,
    subject: String,
    body: String,
    host: String,
    port: u16,
    from: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Sending test email...");
    println!("  From: {}", from);
    println!("  To: {}", to);
    println!("  Subject: {}", subject);
    println!("  Server: {}:{}", host, port);

    // Create SMTP client connection
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let stream = timeout(Duration::from_secs(5), TcpStream::connect(format!("{}:{}", host, port)))
        .await
        .map_err(|_| format!("Failed to connect to SMTP server at {}:{}", host, port))??;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();

    // Read greeting
    timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .map_err(|_| "Timeout reading SMTP greeting")??;

    if !response.starts_with("220") {
        return Err(format!("Unexpected SMTP greeting: {}", response.trim()).into());
    }
    response.clear();

    // EHLO
    writer.write_all(format!("EHLO {}\r\n", host).as_bytes()).await?;
    loop {
        let mut line = String::new();
        timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .map_err(|_| "Timeout reading EHLO response")??;
        response.push_str(&line);
        if line.starts_with("250 ") {
            break;
        }
    }
    response.clear();

    // MAIL FROM
    writer.write_all(format!("MAIL FROM:<{}>\r\n", from).as_bytes()).await?;
    timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .map_err(|_| "Timeout reading MAIL FROM response")??;

    if !response.starts_with("250") {
        return Err(format!("MAIL FROM rejected: {}", response.trim()).into());
    }
    response.clear();

    // RCPT TO
    writer.write_all(format!("RCPT TO:<{}>\r\n", to).as_bytes()).await?;
    timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .map_err(|_| "Timeout reading RCPT TO response")??;

    if !response.starts_with("250") {
        return Err(format!("RCPT TO rejected: {}", response.trim()).into());
    }
    response.clear();

    // DATA
    writer.write_all(b"DATA\r\n").await?;
    timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .map_err(|_| "Timeout reading DATA response")??;

    if !response.starts_with("354") {
        return Err(format!("DATA command rejected: {}", response.trim()).into());
    }
    response.clear();

    // Send email content
    writer.write_all(format!("From: {}\r\n", from).as_bytes()).await?;
    writer.write_all(format!("To: {}\r\n", to).as_bytes()).await?;
    writer.write_all(format!("Subject: {}\r\n", subject).as_bytes()).await?;
    writer.write_all(b"\r\n").await?;
    writer.write_all(format!("{}\r\n", body).as_bytes()).await?;
    writer.write_all(b".\r\n").await?;

    timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .map_err(|_| "Timeout reading message acceptance response")??;

    if !response.starts_with("250") {
        return Err(format!("Message rejected: {}", response.trim()).into());
    }
    response.clear();

    // QUIT
    writer.write_all(b"QUIT\r\n").await?;
    timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .map_err(|_| "Timeout reading QUIT response")??;

    if !response.starts_with("221") {
        return Err(format!("QUIT rejected: {}", response.trim()).into());
    }

    println!("✅ Email sent successfully!");
    Ok(())
}

/// Reload SMTP fixtures from disk
async fn handle_fixtures_reload() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔄 Reloading SMTP fixtures from disk...");

    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.post(format!("{}/smtp/fixtures/reload", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                let count = result["fixtures_loaded"].as_u64().unwrap_or(0);
                println!("✅ Successfully reloaded {} fixtures", count);
            } else {
                println!("❌ Failed to reload fixtures: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running");
        }
    }

    Ok(())
}

/// Validate SMTP fixture file
async fn handle_fixtures_validate(
    file: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Validating SMTP fixture file {}...", file.display());

    // First, try to validate locally by parsing the file
    match std::fs::read_to_string(file) {
        Ok(content) => {
            // Try to parse as YAML first, then JSON
            let parse_result: Result<SmtpFixture, Box<dyn std::error::Error>> =
                if file.extension().and_then(|s| s.to_str()) == Some("json") {
                    serde_json::from_str::<SmtpFixture>(&content)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                } else {
                    serde_yaml::from_str::<SmtpFixture>(&content)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                };

            match parse_result {
                Ok(fixture) => {
                    println!("✅ Fixture file is valid");
                    println!("  Identifier: {}", fixture.identifier);
                    println!("  Name: {}", fixture.name);
                    println!("  Description: {}", fixture.description);
                    println!("  Status Code: {}", fixture.response.status_code);
                    println!("  Match All: {}", fixture.match_criteria.match_all);

                    if let Some(pattern) = &fixture.match_criteria.recipient_pattern {
                        println!("  Recipient Pattern: {}", pattern);
                    }
                    if let Some(pattern) = &fixture.match_criteria.sender_pattern {
                        println!("  Sender Pattern: {}", pattern);
                    }
                    if let Some(pattern) = &fixture.match_criteria.subject_pattern {
                        println!("  Subject Pattern: {}", pattern);
                    }
                }
                Err(e) => {
                    println!("❌ Fixture file is invalid: {}", e);
                    return Ok(());
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to read fixture file: {}", e);
            return Ok(());
        }
    }

    // Also try to validate via management API if server is running
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    let file_content = std::fs::read_to_string(file)?;
    match client
        .post(format!("{}/smtp/fixtures/validate", management_url))
        .body(file_content)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Server validation passed");
            } else {
                let error_msg =
                    response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                println!("⚠️  Server validation failed: {}", error_msg);
            }
        }
        Err(_) => {
            // Server not available, but local validation passed
            println!("💡 Server validation skipped (server not running)");
        }
    }

    Ok(())
}

/// List loaded SMTP fixtures
async fn handle_fixtures_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📋 Listing loaded SMTP fixtures...");

    // Try to connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.get(format!("{}/smtp/fixtures", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let fixtures: Vec<serde_json::Value> = response.json().await?;
                if fixtures.is_empty() {
                    println!("📋 No fixtures loaded");
                } else {
                    println!("📋 Found {} fixtures:", fixtures.len());
                    println!("{:<20} {:<50} Description", "Identifier", "Name");
                    println!("{}", "-".repeat(100));

                    for fixture in fixtures {
                        let identifier = fixture["identifier"].as_str().unwrap_or("N/A");
                        let name = fixture["name"].as_str().unwrap_or("N/A");
                        let description = fixture["description"].as_str().unwrap_or("");

                        // Truncate name if too long
                        let name_display = if name.len() > 47 {
                            format!("{}...", &name[..44])
                        } else {
                            name.to_string()
                        };

                        println!(
                            "{:<20} {:<50} {}",
                            &identifier[..std::cmp::min(identifier.len(), 20)],
                            name_display,
                            description
                        );
                    }
                }
            } else {
                println!("❌ Failed to access fixtures: HTTP {}", response.status());
                println!("💡 Make sure MockForge server is running with SMTP enabled");
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to MockForge management API: {}", e);
            println!("💡 Make sure MockForge server is running at {}", management_url);
            println!("💡 Or set MOCKFORGE_MANAGEMENT_URL environment variable");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_mailbox_command_construction() {
        let cmd = SmtpCommands::Mailbox {
            mailbox_command: MailboxCommands::List,
        };
        assert!(matches!(cmd, SmtpCommands::Mailbox { .. }));
    }

    #[test]
    fn test_smtp_fixtures_command_construction() {
        let cmd = SmtpCommands::Fixtures {
            fixtures_command: FixturesCommands::List,
        };
        assert!(matches!(cmd, SmtpCommands::Fixtures { .. }));
    }

    #[test]
    fn test_smtp_send_command_construction() {
        let cmd = SmtpCommands::Send {
            to: "user@example.com".to_string(),
            subject: "Test".to_string(),
            body: "Hello".to_string(),
            host: "localhost".to_string(),
            port: 1025,
            from: "test@mockforge.cli".to_string(),
        };
        assert!(matches!(cmd, SmtpCommands::Send { .. }));
    }

    #[test]
    fn test_mailbox_list_command() {
        let _cmd = MailboxCommands::List;
    }

    #[test]
    fn test_mailbox_clear_command() {
        let _cmd = MailboxCommands::Clear;
    }

    #[test]
    fn test_mailbox_show_command() {
        let _cmd = MailboxCommands::Show {
            email_id: "test-id".to_string(),
        };
    }

    #[test]
    fn test_mailbox_export_command() {
        let _cmd = MailboxCommands::Export {
            format: "json".to_string(),
            output: std::path::PathBuf::from("emails.json"),
        };
    }
}

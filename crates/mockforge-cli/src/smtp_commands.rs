//! SMTP server management and mailbox operations

use crate::{FixturesCommands, MailboxCommands, SmtpCommands};

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
                    println!("{:<5} {:<30} {:<50} {}", "ID", "From", "Subject", "Received");
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

/// Handle fixture management commands
async fn handle_fixtures_command(
    fixtures_command: FixturesCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match fixtures_command {
        FixturesCommands::List => {
            println!("📋 Listing loaded fixtures...");
            // TODO: Implement fixture listing
            println!(
                "Fixture listing not yet implemented. Start MockForge server to access fixtures."
            );
        }
        FixturesCommands::Reload => {
            println!("🔄 Reloading fixtures from disk...");
            // TODO: Implement fixture reloading
            println!(
                "Fixture reloading not yet implemented. Start MockForge server to access fixtures."
            );
        }
        FixturesCommands::Validate { file } => {
            println!("🔍 Validating fixture file {}...", file.display());
            // TODO: Implement fixture validation
            println!("Fixture validation not yet implemented.");
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

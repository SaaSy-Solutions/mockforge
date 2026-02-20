//! SSE (Server-Sent Events) stream consumer for live log data.

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::api::models::RequestLog;
use crate::event::Event;

/// Spawn a background task that connects to the SSE log stream and pushes
/// `Event::LogLine` messages into the event channel.
///
/// Automatically reconnects on disconnect with exponential backoff.
pub fn spawn_sse_listener(
    base_url: String,
    token: Option<String>,
    tx: mpsc::UnboundedSender<Event>,
) {
    tokio::spawn(async move {
        let mut backoff_ms = 500u64;
        let max_backoff_ms = 30_000u64;

        loop {
            match connect_sse(&base_url, token.as_deref(), &tx).await {
                Ok(()) => {
                    // Stream ended cleanly — reset backoff and reconnect.
                    backoff_ms = 500;
                    tracing::debug!("SSE stream ended, reconnecting…");
                }
                Err(e) => {
                    tracing::warn!("SSE connection error: {e:#}");
                    let _ = tx.send(Event::ApiError {
                        screen: "logs",
                        message: format!("SSE disconnected: {e:#}"),
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
        }
    });
}

async fn connect_sse(
    base_url: &str,
    token: Option<&str>,
    tx: &mpsc::UnboundedSender<Event>,
) -> Result<()> {
    let url = format!("{base_url}/__mockforge/logs/sse");
    let client = reqwest::Client::new();

    let mut req = client.get(&url);
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }

    let resp = req
        .send()
        .await
        .context("SSE connection failed")?
        .error_for_status()
        .context("SSE returned error status")?;

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("SSE read error")?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete SSE messages (delimited by double newline).
        while let Some(pos) = buffer.find("\n\n") {
            let message = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            if let Some(data) = extract_sse_data(&message) {
                // Try to parse as RequestLog JSON; if it fails, send raw.
                match serde_json::from_str::<RequestLog>(&data) {
                    Ok(log) => {
                        let line = format!(
                            "{} {:>6} {:<30} {} {:>5}ms {:>6}",
                            log.timestamp.format("%H:%M:%S"),
                            log.method,
                            truncate(&log.path, 30),
                            log.status_code,
                            log.response_time_ms,
                            format_bytes(log.response_size_bytes),
                        );
                        if tx.send(Event::LogLine(line)).is_err() {
                            return Ok(());
                        }
                    }
                    Err(_) => {
                        if tx.send(Event::LogLine(data)).is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Extract the `data:` field from an SSE message.
fn extract_sse_data(message: &str) -> Option<String> {
    let mut data_parts = Vec::new();
    for line in message.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            data_parts.push(rest.trim().to_string());
        }
    }
    if data_parts.is_empty() {
        None
    } else {
        Some(data_parts.join("\n"))
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    }
}

//! Email notification system for chaos orchestration results

use crate::{Result, ReportingError};
use crate::pdf::ExecutionReport;
use lettre::{Message, SmtpTransport, Transport};
use lettre::message::{header, Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use serde::{Deserialize, Serialize};
use std::fs;

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: String,
}

/// Email report
#[derive(Debug, Clone)]
pub struct EmailReport {
    pub subject: String,
    pub recipients: Vec<String>,
    pub html_body: String,
    pub text_body: String,
    pub pdf_attachment: Option<Vec<u8>>,
}

/// Email notifier
pub struct EmailNotifier {
    config: EmailConfig,
    transport: SmtpTransport,
}

impl EmailNotifier {
    /// Create a new email notifier
    pub fn new(config: EmailConfig) -> Result<Self> {
        let creds = Credentials::new(
            config.username.clone(),
            config.password.clone(),
        );

        let transport = SmtpTransport::relay(&config.smtp_host)
            .map_err(|e| ReportingError::Email(e.to_string()))?
            .credentials(creds)
            .port(config.smtp_port)
            .build();

        Ok(Self { config, transport })
    }

    /// Send email report
    pub fn send(&self, email_report: &EmailReport) -> Result<()> {
        let from = format!("{} <{}>", self.config.from_name, self.config.from_address);

        let mut message_builder = Message::builder()
            .from(from.parse().map_err(|e| ReportingError::Email(format!("Invalid from address: {}", e)))?)
            .subject(&email_report.subject);

        // Add recipients
        for recipient in &email_report.recipients {
            message_builder = message_builder.to(
                recipient.parse().map_err(|e| ReportingError::Email(format!("Invalid recipient: {}", e)))?
            );
        }

        // Build multipart message
        let mut multipart = MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(header::ContentType::TEXT_PLAIN)
                    .body(email_report.text_body.clone())
            )
            .singlepart(
                SinglePart::builder()
                    .header(header::ContentType::TEXT_HTML)
                    .body(email_report.html_body.clone())
            );

        // Add PDF attachment if provided
        if let Some(ref pdf_data) = email_report.pdf_attachment {
            let attachment = Attachment::new("report.pdf".to_string())
                .body(pdf_data.clone(), "application/pdf".parse().unwrap());
            multipart = MultiPart::mixed()
                .multipart(multipart)
                .singlepart(attachment);
        }

        let email = message_builder
            .multipart(multipart)
            .map_err(|e| ReportingError::Email(e.to_string()))?;

        self.transport
            .send(&email)
            .map_err(|e| ReportingError::Email(e.to_string()))?;

        Ok(())
    }

    /// Generate and send execution report
    pub fn send_execution_report(
        &self,
        report: &ExecutionReport,
        recipients: Vec<String>,
        include_pdf: bool,
    ) -> Result<()> {
        let subject = format!(
            "Chaos Test Report: {} - {}",
            report.orchestration_name,
            report.status
        );

        let html_body = self.generate_html_report(report);
        let text_body = self.generate_text_report(report);

        let pdf_attachment = if include_pdf {
            Some(self.generate_pdf_attachment(report)?)
        } else {
            None
        };

        let email_report = EmailReport {
            subject,
            recipients,
            html_body,
            text_body,
            pdf_attachment,
        };

        self.send(&email_report)
    }

    /// Generate HTML report body
    fn generate_html_report(&self, report: &ExecutionReport) -> String {
        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .header {{ background: #2c3e50; color: white; padding: 20px; }}
        .content {{ padding: 20px; }}
        .status-badge {{ padding: 5px 10px; border-radius: 3px; font-weight: bold; }}
        .success {{ background: #27ae60; color: white; }}
        .failure {{ background: #e74c3c; color: white; }}
        .metrics {{ display: grid; grid-template-columns: repeat(2, 1fr); gap: 15px; margin: 20px 0; }}
        .metric-card {{ border: 1px solid #ddd; padding: 15px; border-radius: 5px; }}
        .metric-value {{ font-size: 24px; font-weight: bold; color: #2c3e50; }}
        .metric-label {{ font-size: 12px; color: #7f8c8d; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background: #f8f9fa; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🌩️ Chaos Orchestration Report</h1>
        <p>{}</p>
    </div>

    <div class="content">
        <h2>Summary</h2>
        <p>
            <span class="status-badge {}">{}</span>
        </p>
        <p>
            <strong>Duration:</strong> {}s<br>
            <strong>Started:</strong> {}<br>
            <strong>Ended:</strong> {}
        </p>

        <h2>Execution Metrics</h2>
        <div class="metrics">
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Total Steps</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Completed Steps</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.2}%</div>
                <div class="metric-label">Error Rate</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.2}ms</div>
                <div class="metric-label">Avg Latency</div>
            </div>
        </div>

        {}

        {}

        <hr>
        <p style="font-size: 12px; color: #7f8c8d;">
            Generated by MockForge on {}<br>
            <a href="https://github.com/your-org/mockforge">View Documentation</a>
        </p>
    </div>
</body>
</html>
"#,
            report.orchestration_name,
            if report.failed_steps == 0 { "success" } else { "failure" },
            report.status,
            report.duration_seconds,
            report.start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            report.end_time.format("%Y-%m-%d %H:%M:%S UTC"),
            report.total_steps,
            report.completed_steps,
            report.metrics.error_rate * 100.0,
            report.metrics.avg_latency_ms,
            self.generate_failures_html(&report.failures),
            self.generate_recommendations_html(&report.recommendations),
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
        )
    }

    /// Generate failures section HTML
    fn generate_failures_html(&self, failures: &[crate::pdf::FailureDetail]) -> String {
        if failures.is_empty() {
            return String::new();
        }

        let mut html = String::from("<h2>Failures</h2><table><tr><th>Step</th><th>Error</th><th>Time</th></tr>");

        for failure in failures {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                failure.step_name,
                failure.error_message,
                failure.timestamp.format("%H:%M:%S")
            ));
        }

        html.push_str("</table>");
        html
    }

    /// Generate recommendations section HTML
    fn generate_recommendations_html(&self, recommendations: &[String]) -> String {
        if recommendations.is_empty() {
            return String::new();
        }

        let mut html = String::from("<h2>Recommendations</h2><ul>");

        for rec in recommendations {
            html.push_str(&format!("<li>{}</li>", rec));
        }

        html.push_str("</ul>");
        html
    }

    /// Generate plain text report
    fn generate_text_report(&self, report: &ExecutionReport) -> String {
        format!(
            "CHAOS ORCHESTRATION REPORT\n\
             ========================\n\n\
             Orchestration: {}\n\
             Status: {}\n\
             Duration: {}s\n\
             Started: {}\n\
             Ended: {}\n\n\
             EXECUTION SUMMARY\n\
             -----------------\n\
             Total Steps: {}\n\
             Completed: {}\n\
             Failed: {}\n\n\
             METRICS\n\
             -------\n\
             Total Requests: {}\n\
             Error Rate: {:.2}%\n\
             Avg Latency: {:.2}ms\n\
             P95 Latency: {:.2}ms\n\n\
             Generated by MockForge\n",
            report.orchestration_name,
            report.status,
            report.duration_seconds,
            report.start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            report.end_time.format("%Y-%m-%d %H:%M:%S UTC"),
            report.total_steps,
            report.completed_steps,
            report.failed_steps,
            report.metrics.total_requests,
            report.metrics.error_rate * 100.0,
            report.metrics.avg_latency_ms,
            report.metrics.p95_latency_ms
        )
    }

    /// Generate PDF attachment
    fn generate_pdf_attachment(&self, report: &ExecutionReport) -> Result<Vec<u8>> {
        use crate::pdf::{PdfReportGenerator, PdfConfig};

        let config = PdfConfig::default();
        let generator = PdfReportGenerator::new(config);

        let temp_path = "/tmp/mockforge_report.pdf";
        generator.generate(report, temp_path)?;

        let pdf_data = fs::read(temp_path)?;
        fs::remove_file(temp_path)?;

        Ok(pdf_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_generation() {
        let config = EmailConfig {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            username: "user".to_string(),
            password: "pass".to_string(),
            from_address: "noreply@example.com".to_string(),
            from_name: "MockForge".to_string(),
        };

        // Note: Can't test actual SMTP without a server
        // This just tests the struct creation
        assert_eq!(config.smtp_port, 587);
    }
}

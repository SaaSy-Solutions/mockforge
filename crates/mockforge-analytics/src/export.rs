//! Data export functionality

use crate::database::AnalyticsDatabase;
use crate::error::Result;
use crate::models::AnalyticsFilter;
use std::io::Write;

impl AnalyticsDatabase {
    /// Export metrics to CSV format
    pub async fn export_to_csv<W: Write>(
        &self,
        writer: &mut W,
        filter: &AnalyticsFilter,
    ) -> Result<usize> {
        // Write CSV header
        writeln!(
            writer,
            "timestamp,protocol,method,endpoint,status_code,request_count,error_count,avg_latency_ms,p95_latency_ms,bytes_sent,bytes_received"
        )?;

        let aggregates = self.get_minute_aggregates(filter).await?;

        for agg in &aggregates {
            let avg_latency = if agg.request_count > 0 {
                agg.latency_sum / agg.request_count as f64
            } else {
                0.0
            };

            writeln!(
                writer,
                "{},{},{},{},{},{},{},{:.2},{:.2},{},{}",
                agg.timestamp,
                agg.protocol,
                agg.method.as_deref().unwrap_or(""),
                agg.endpoint.as_deref().unwrap_or(""),
                agg.status_code.unwrap_or(0),
                agg.request_count,
                agg.error_count,
                avg_latency,
                agg.latency_p95.unwrap_or(0.0),
                agg.bytes_sent,
                agg.bytes_received
            )?;
        }

        Ok(aggregates.len())
    }

    /// Export metrics to JSON format
    pub async fn export_to_json(&self, filter: &AnalyticsFilter) -> Result<String> {
        let aggregates = self.get_minute_aggregates(filter).await?;
        let json = serde_json::to_string_pretty(&aggregates)?;
        Ok(json)
    }

    /// Export endpoint stats to CSV
    pub async fn export_endpoints_to_csv<W: Write>(
        &self,
        writer: &mut W,
        workspace_id: Option<&str>,
        limit: i64,
    ) -> Result<usize> {
        writeln!(
            writer,
            "endpoint,protocol,method,total_requests,total_errors,error_rate,avg_latency_ms,p95_latency_ms,bytes_sent,bytes_received"
        )?;

        let endpoints = self.get_top_endpoints(limit, workspace_id).await?;

        for ep in &endpoints {
            let error_rate = if ep.total_requests > 0 {
                (ep.total_errors as f64 / ep.total_requests as f64) * 100.0
            } else {
                0.0
            };

            writeln!(
                writer,
                "{},{},{},{},{},{:.2},{:.2},{:.2},{},{}",
                ep.endpoint,
                ep.protocol,
                ep.method.as_deref().unwrap_or(""),
                ep.total_requests,
                ep.total_errors,
                error_rate,
                ep.avg_latency_ms.unwrap_or(0.0),
                ep.p95_latency_ms.unwrap_or(0.0),
                ep.total_bytes_sent,
                ep.total_bytes_received
            )?;
        }

        Ok(endpoints.len())
    }

    /// Export error events to CSV
    pub async fn export_errors_to_csv<W: Write>(
        &self,
        writer: &mut W,
        filter: &AnalyticsFilter,
        limit: i64,
    ) -> Result<usize> {
        writeln!(
            writer,
            "timestamp,protocol,method,endpoint,status_code,error_type,error_category,error_message,client_ip,trace_id"
        )?;

        let errors = self.get_recent_errors(limit, filter).await?;

        for err in &errors {
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{},{},{}",
                err.timestamp,
                err.protocol,
                err.method.as_deref().unwrap_or(""),
                err.endpoint.as_deref().unwrap_or(""),
                err.status_code.unwrap_or(0),
                err.error_type.as_deref().unwrap_or(""),
                err.error_category.as_deref().unwrap_or(""),
                err.error_message.as_deref().unwrap_or(""),
                err.client_ip.as_deref().unwrap_or(""),
                err.trace_id.as_deref().unwrap_or("")
            )?;
        }

        Ok(errors.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_export_to_csv() {
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        db.run_migrations().await.unwrap();

        let mut buffer = Vec::new();
        let filter = AnalyticsFilter::default();

        let count = db.export_to_csv(&mut buffer, &filter).await.unwrap();
        assert_eq!(count, 0); // No data yet

        let csv = String::from_utf8(buffer).unwrap();
        assert!(csv.contains("timestamp,protocol"));
    }
}

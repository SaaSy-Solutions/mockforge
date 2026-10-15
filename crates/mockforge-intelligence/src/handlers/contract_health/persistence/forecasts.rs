//! Forecast selection and typed conversion of probability and timing fields.

use super::super::TimelineEvent;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, Row};

pub(super) async fn events(
    pool: &sqlx::PgPool,
    workspace: Option<uuid::Uuid>,
) -> Vec<TimelineEvent> {
    sqlx::query(
        "SELECT id, service_id, service_name, endpoint, method, forecast_window_days,
         predicted_change_probability, predicted_break_probability, next_expected_change_date,
         confidence, predicted_at
         FROM api_change_forecasts
         WHERE workspace_id = $1 OR workspace_id IS NULL
         ORDER BY predicted_at DESC LIMIT 50",
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter_map(event)
    .collect()
}

fn event(row: PgRow) -> Option<TimelineEvent> {
    let id: uuid::Uuid = row.try_get("id").ok()?;
    let endpoint: String = row.try_get("endpoint").ok()?;
    let method: String = row.try_get("method").ok()?;
    let window: i32 = row.try_get("forecast_window_days").ok()?;
    let change_probability: f64 = row.try_get("predicted_change_probability").ok()?;
    let break_probability: f64 = row.try_get("predicted_break_probability").ok()?;
    let confidence: f64 = row.try_get("confidence").ok()?;
    let predicted_at: DateTime<Utc> = row.try_get("predicted_at").ok()?;
    let next: Option<DateTime<Utc>> = row.try_get("next_expected_change_date").ok();
    Some(TimelineEvent::Forecast {
        id: id.to_string(),
        endpoint,
        method,
        window_days: window as u32,
        change_probability,
        break_probability,
        next_expected_change: next.map(|date| date.timestamp()),
        confidence,
        predicted_at: predicted_at.timestamp(),
    })
}

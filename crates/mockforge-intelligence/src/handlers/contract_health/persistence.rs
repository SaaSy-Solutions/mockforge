//! Database timeline assembly keeps each schema's conversion independent.

use super::TimelineEvent;
mod forecasts;
mod threats;

pub(super) async fn events(
    pool: &sqlx::PgPool,
    workspace: Option<uuid::Uuid>,
) -> Vec<TimelineEvent> {
    let mut events = threats::events(pool, workspace).await;
    events.extend(forecasts::events(pool, workspace).await);
    events
}

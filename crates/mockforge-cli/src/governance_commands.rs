//! Governance Commands
//!
//! CLI commands for API governance features:
//! - Forecasting API changes
//! - Semantic drift analysis
//! - Threat assessment
//! - Contract health status

use mockforge_core::{
    contract_drift::forecasting::{Forecaster, ForecastingConfig},
    contract_drift::threat_modeling::{ThreatAnalyzer, ThreatModelingConfig, ThreatAssessment, ThreatLevel},
    incidents::types::{DriftIncident, IncidentType, IncidentSeverity, IncidentStatus},
    openapi::OpenApiSpec,
    Error, Result,
};
use std::path::PathBuf;
use tracing::{error, info, warn};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

/// Connect to database if DATABASE_URL is set
async fn connect_database() -> Option<PgPool> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            warn!("DATABASE_URL not set, skipping database queries");
            return None;
        }
    };

    match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            info!("Connected to database for governance queries");
            Some(pool)
        }
        Err(e) => {
            warn!("Failed to connect to database: {}. Continuing without database support.", e);
            None
        }
    }
}

/// Query drift incidents from database
async fn query_drift_incidents(
    pool: &PgPool,
    workspace_id: Option<&str>,
    service_id: Option<&str>,
    endpoint: Option<&str>,
    method: Option<&str>,
    window_days: u32,
) -> Result<Vec<DriftIncident>> {
    let window_start = Utc::now() - chrono::Duration::days(window_days as i64);

    let mut query = String::from(
        "SELECT id, workspace_id, endpoint, method, incident_type, severity, status,
         detected_at, details, created_at, updated_at, resolved_at, budget_id,
         sync_cycle_id, contract_diff_id, before_sample, after_sample,
         fitness_test_results, affected_consumers, protocol
         FROM drift_incidents
         WHERE detected_at >= $1",
    );

    let mut bind_index = 2;

    if let Some(ws_id) = workspace_id {
        query.push_str(&format!(" AND workspace_id = ${}", bind_index));
        bind_index += 1;
    }

    if let Some(ep) = endpoint {
        query.push_str(&format!(" AND endpoint = ${}", bind_index));
        bind_index += 1;
    }

    if let Some(m) = method {
        query.push_str(&format!(" AND method = ${}", bind_index));
        bind_index += 1;
    }

    query.push_str(" ORDER BY detected_at DESC");

    let mut query_builder = sqlx::query(&query).bind(window_start);

    if let Some(ws_id) = workspace_id {
        let uuid = Uuid::parse_str(ws_id).ok();
        query_builder = query_builder.bind(uuid);
    }
    if let Some(ep) = endpoint {
        query_builder = query_builder.bind(ep);
    }
    if let Some(m) = method {
        query_builder = query_builder.bind(m);
    }

    let rows = query_builder
        .fetch_all(pool)
        .await
        .map_err(|e| Error::generic(&format!("Failed to query drift incidents: {}", e)))?;

    let mut incidents = Vec::new();
    for row in rows {
        match map_row_to_drift_incident(&row) {
            Ok(incident) => incidents.push(incident),
            Err(e) => {
                warn!("Failed to map drift incident row: {}", e);
                continue;
            }
        }
    }

    Ok(incidents)
}

/// Map database row to DriftIncident
fn map_row_to_drift_incident(row: &sqlx::postgres::PgRow) -> Result<DriftIncident> {
    use sqlx::Row;

    let id: Uuid = row.try_get("id")
        .map_err(|e| Error::generic(&format!("Failed to get id: {}", e)))?;
    let workspace_id: Option<Uuid> = row.try_get("workspace_id").ok();
    let endpoint: String = row.try_get("endpoint")
        .map_err(|e| Error::generic(&format!("Failed to get endpoint: {}", e)))?;
    let method: String = row.try_get("method")
        .map_err(|e| Error::generic(&format!("Failed to get method: {}", e)))?;
    let incident_type_str: String = row.try_get("incident_type")
        .map_err(|e| Error::generic(&format!("Failed to get incident_type: {}", e)))?;
    let severity_str: String = row.try_get("severity")
        .map_err(|e| Error::generic(&format!("Failed to get severity: {}", e)))?;
    let status_str: String = row.try_get("status")
        .map_err(|e| Error::generic(&format!("Failed to get status: {}", e)))?;
    let detected_at: DateTime<Utc> = row.try_get("detected_at")
        .map_err(|e| Error::generic(&format!("Failed to get detected_at: {}", e)))?;
    let details: serde_json::Value = row.try_get("details").unwrap_or_default();
    let created_at: DateTime<Utc> = row.try_get("created_at")
        .map_err(|e| Error::generic(&format!("Failed to get created_at: {}", e)))?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at")
        .map_err(|e| Error::generic(&format!("Failed to get updated_at: {}", e)))?;
    let resolved_at: Option<DateTime<Utc>> = row.try_get("resolved_at").ok();
    let budget_id: Option<Uuid> = row.try_get("budget_id").ok();
    let sync_cycle_id: Option<String> = row.try_get("sync_cycle_id").ok();
    let contract_diff_id: Option<String> = row.try_get("contract_diff_id").ok();
    let before_sample: Option<serde_json::Value> = row.try_get("before_sample").ok();
    let after_sample: Option<serde_json::Value> = row.try_get("after_sample").ok();
    let fitness_test_results: Option<serde_json::Value> = row.try_get("fitness_test_results").ok();
    let affected_consumers: Option<serde_json::Value> = row.try_get("affected_consumers").ok();
    let protocol: Option<String> = row.try_get("protocol").ok();

    let incident_type = match incident_type_str.as_str() {
        "breaking_change" => IncidentType::BreakingChange,
        "threshold_exceeded" => IncidentType::ThresholdExceeded,
        _ => return Err(Error::generic(&format!("Invalid incident_type: {}", incident_type_str))),
    };

    let severity = match severity_str.as_str() {
        "low" => IncidentSeverity::Low,
        "medium" => IncidentSeverity::Medium,
        "high" => IncidentSeverity::High,
        "critical" => IncidentSeverity::Critical,
        _ => return Err(Error::generic(&format!("Invalid severity: {}", severity_str))),
    };

    let status = match status_str.as_str() {
        "open" => IncidentStatus::Open,
        "acknowledged" => IncidentStatus::Acknowledged,
        "resolved" => IncidentStatus::Resolved,
        "closed" => IncidentStatus::Closed,
        _ => return Err(Error::generic(&format!("Invalid status: {}", status_str))),
    };

    // Parse fitness_test_results JSONB array
    let fitness_results = if let Some(json) = fitness_test_results {
        serde_json::from_value(json).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Parse affected_consumers JSONB object
    let consumers = if let Some(json) = affected_consumers {
        serde_json::from_value(json).ok()
    } else {
        None
    };

    // Parse protocol
    let protocol_enum = protocol.and_then(|p| {
        match p.as_str() {
            "http" => Some(mockforge_core::protocol_abstraction::Protocol::Http),
            "grpc" => Some(mockforge_core::protocol_abstraction::Protocol::Grpc),
            "websocket" => Some(mockforge_core::protocol_abstraction::Protocol::WebSocket),
            "mqtt" => Some(mockforge_core::protocol_abstraction::Protocol::Mqtt),
            "kafka" => Some(mockforge_core::protocol_abstraction::Protocol::Kafka),
            _ => None,
        }
    });

    Ok(DriftIncident {
        id: id.to_string(),
        budget_id: budget_id.map(|u| u.to_string()),
        workspace_id: workspace_id.map(|u| u.to_string()),
        endpoint,
        method,
        incident_type,
        severity,
        status,
        detected_at: detected_at.timestamp(),
        resolved_at: resolved_at.map(|dt| dt.timestamp()),
        details,
        external_ticket_id: None, // Not in query, can be added if needed
        external_ticket_url: None, // Not in query, can be added if needed
        created_at: created_at.timestamp(),
        updated_at: updated_at.timestamp(),
        sync_cycle_id,
        contract_diff_id,
        before_sample,
        after_sample,
        fitness_test_results: fitness_results,
        affected_consumers: consumers,
        protocol: protocol_enum,
    })
}

/// Handle the forecast generate command
pub async fn handle_forecast_generate(
    workspace_id: Option<String>,
    service_id: Option<String>,
    endpoint: Option<String>,
    method: Option<String>,
    window_days: Option<u32>,
) -> Result<()> {
    info!("Generating API change forecasts");

    let window = window_days.unwrap_or(90);
    if !matches!(window, 30 | 90 | 180) {
        return Err(Error::generic("Window must be 30, 90, or 180 days"));
    }

    let config = ForecastingConfig::default();
    let forecaster = Forecaster::new(config);

    // Query historical incidents from database
    let incidents = if let Some(pool) = connect_database().await {
        match query_drift_incidents(
            &pool,
            workspace_id.as_deref(),
            service_id.as_deref(),
            endpoint.as_deref(),
            method.as_deref(),
            window,
        )
        .await
        {
            Ok(incidents) => {
                info!("Found {} historical incidents in database", incidents.len());
                incidents
            }
            Err(e) => {
                warn!("Failed to query incidents: {}. Continuing without database data.", e);
                Vec::new()
            }
        }
    } else {
        warn!("Database not available, cannot generate forecasts from historical data");
        Vec::new()
    };

    if incidents.is_empty() {
        warn!("No historical incidents found. Forecast generation requires incident data.");
        info!("Window: {} days", window);
        if let Some(ws_id) = workspace_id {
            info!("Workspace: {}", ws_id);
        }
        if let Some(svc_id) = service_id {
            info!("Service: {}", svc_id);
        }
        if let Some(ep) = endpoint {
            info!("Endpoint: {}", ep);
        }
        if let Some(m) = method {
            info!("Method: {}", m);
        }
        return Ok(());
    }

    // Generate forecast for each unique endpoint/method combination
    use std::collections::HashMap;
    let mut endpoint_groups: HashMap<(String, String), Vec<&DriftIncident>> = HashMap::new();

    for incident in &incidents {
        let key = (incident.endpoint.clone(), incident.method.clone());
        endpoint_groups.entry(key).or_insert_with(Vec::new).push(incident);
    }

    info!("Generating forecasts for {} endpoint(s)...", endpoint_groups.len());

    for ((endpoint, method), group_incidents) in endpoint_groups {
        let incidents_slice: Vec<DriftIncident> = group_incidents.iter().map(|i| (*i).clone()).collect();

        if let Some(forecast) = forecaster.generate_forecast(
            &incidents_slice,
            workspace_id.clone(),
            service_id.clone(),
            None, // service_name
            endpoint.clone(),
            method.clone(),
            window,
        ) {
            info!("Forecast for {} {}:", method, endpoint);
            info!("  Change Probability: {:.1}%", forecast.predicted_change_probability * 100.0);
            info!("  Break Probability: {:.1}%", forecast.predicted_break_probability * 100.0);
            info!("  Confidence: {:.1}%", forecast.confidence * 100.0);
            info!("  Volatility Score: {:.2}", forecast.volatility_score);
            if let Some(next_change) = forecast.next_expected_change_date {
                info!("  Next Expected Change: {}", next_change.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            if let Some(next_break) = forecast.next_expected_break_date {
                info!("  Next Expected Break: {}", next_break.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            if !forecast.seasonal_patterns.is_empty() {
                info!("  Seasonal Patterns: {}", forecast.seasonal_patterns.len());
            }
        } else {
            warn!("Could not generate forecast for {} {} (insufficient data)", method, endpoint);
        }
    }

    info!("Forecast generation completed");
    Ok(())
}

/// Handle the semantic analyze command
pub async fn handle_semantic_analyze(
    before_spec_path: PathBuf,
    after_spec_path: PathBuf,
    endpoint: String,
    method: String,
    output: Option<PathBuf>,
) -> Result<()> {
    info!("Analyzing semantic drift between contract versions");

    let before_spec = OpenApiSpec::from_file(&before_spec_path).await?;
    let after_spec = OpenApiSpec::from_file(&after_spec_path).await?;

    info!("Loaded before spec from: {:?}", before_spec_path);
    info!("Loaded after spec from: {:?}", after_spec_path);

    // Create analyzer
    let config = mockforge_core::ai_contract_diff::ContractDiffConfig::default();
    let analyzer = mockforge_core::ai_contract_diff::ContractDiffAnalyzer::new(config)?;

    // Run semantic analysis
    info!("Analyzing semantic drift for {} {}...", method, endpoint);
    let result = analyzer
        .compare_specs(&before_spec, &after_spec, &endpoint, &method)
        .await?;

    if let Some(semantic_result) = result {
        info!("Semantic drift detected!");
        info!("  Change Type: {:?}", semantic_result.change_type);
        info!("  Semantic Confidence: {:.2}", semantic_result.semantic_confidence);
        info!("  Soft-Breaking Score: {:.2}", semantic_result.soft_breaking_score);

        if let Some(output_path) = output {
            let output_json = serde_json::to_string_pretty(&semantic_result)?;
            std::fs::write(&output_path, output_json)?;
            info!("Results written to: {:?}", output_path);
        } else {
            println!("{}", serde_json::to_string_pretty(&semantic_result)?);
        }

        if semantic_result.soft_breaking_score >= 0.65 {
            warn!("High soft-breaking score detected - this may break clients!");
            std::process::exit(1);
        }
    } else {
        info!("No significant semantic drift detected");
    }

    Ok(())
}

/// Handle the threat assess command
pub async fn handle_threat_assess(
    spec_path: PathBuf,
    workspace_id: Option<String>,
    service_id: Option<String>,
    endpoint: Option<String>,
    method: Option<String>,
    output: Option<PathBuf>,
) -> Result<()> {
    info!("Assessing contract security threats");

    let spec = OpenApiSpec::from_file(&spec_path).await?;
    info!("Loaded spec from: {:?}", spec_path);

    let config = ThreatModelingConfig::default();
    let analyzer = ThreatAnalyzer::new(config)?;

    info!("Running threat analysis...");
    let assessment = analyzer
        .analyze_contract(
            &spec,
            workspace_id,
            service_id,
            None, // service_name
            endpoint,
            method,
        )
        .await?;

    info!("Threat Assessment Complete");
    info!("  Threat Level: {:?}", assessment.threat_level);
    info!("  Threat Score: {:.2}", assessment.threat_score);
    info!("  Findings: {}", assessment.findings.len());
    info!("  Remediations: {}", assessment.remediation_suggestions.len());

    // Print findings
    for finding in &assessment.findings {
        warn!(
            "  [{}] {}: {}",
            format!("{:?}", finding.severity),
            format!("{:?}", finding.finding_type),
            finding.description
        );
    }

    if let Some(output_path) = output {
        let output_json = serde_json::to_string_pretty(&assessment)?;
        std::fs::write(&output_path, output_json)?;
        info!("Results written to: {:?}", output_path);
    } else {
        println!("{}", serde_json::to_string_pretty(&assessment)?);
    }

    // Exit with error code if critical threats found
    if matches!(assessment.threat_level, mockforge_core::contract_drift::threat_modeling::ThreatLevel::Critical) {
        error!("Critical threats detected!");
        std::process::exit(1);
    }

    Ok(())
}

/// Query semantic incidents from database
async fn query_semantic_incidents(
    pool: &PgPool,
    workspace_id: Option<&str>,
    days: i64,
) -> Result<Vec<mockforge_core::incidents::semantic_manager::SemanticIncident>> {
    use mockforge_core::ai_contract_diff::SemanticChangeType;
    use mockforge_core::incidents::semantic_manager::SemanticIncident;
    use mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus};

    let window_start = Utc::now() - chrono::Duration::days(days);

    let mut query = String::from(
        "SELECT id, workspace_id, endpoint, method, semantic_change_type, severity, status,
         semantic_confidence, soft_breaking_score, llm_analysis, before_semantic_state,
         after_semantic_state, details, related_drift_incident_id, contract_diff_id,
         external_ticket_id, external_ticket_url, detected_at, created_at, acknowledged_at,
         resolved_at, closed_at, updated_at
         FROM semantic_drift_incidents
         WHERE detected_at >= $1",
    );

    let mut bind_index = 2;

    if let Some(ws_id) = workspace_id {
        query.push_str(&format!(" AND workspace_id = ${}", bind_index));
        bind_index += 1;
    }

    query.push_str(" ORDER BY detected_at DESC LIMIT 100");

    let mut query_builder = sqlx::query(&query).bind(window_start);

    if let Some(ws_id) = workspace_id {
        let uuid = Uuid::parse_str(ws_id).ok();
        query_builder = query_builder.bind(uuid);
    }

    let rows = query_builder
        .fetch_all(pool)
        .await
        .map_err(|e| Error::generic(&format!("Failed to query semantic incidents: {}", e)))?;

    let mut incidents = Vec::new();
    for row in rows {
        use sqlx::Row;

        let id: Uuid = row.try_get("id")
            .map_err(|e| Error::generic(&format!("Failed to get id: {}", e)))?;
        let workspace_id: Option<Uuid> = row.try_get("workspace_id").ok();
        let endpoint: String = row.try_get("endpoint")
            .map_err(|e| Error::generic(&format!("Failed to get endpoint: {}", e)))?;
        let method: String = row.try_get("method")
            .map_err(|e| Error::generic(&format!("Failed to get method: {}", e)))?;
        let change_type_str: String = row.try_get("semantic_change_type")
            .map_err(|e| Error::generic(&format!("Failed to get semantic_change_type: {}", e)))?;
        let severity_str: String = row.try_get("severity")
            .map_err(|e| Error::generic(&format!("Failed to get severity: {}", e)))?;
        let status_str: String = row.try_get("status")
            .map_err(|e| Error::generic(&format!("Failed to get status: {}", e)))?;
        let semantic_confidence: f64 = row.try_get("semantic_confidence")
            .map_err(|e| Error::generic(&format!("Failed to get semantic_confidence: {}", e)))?;
        let soft_breaking_score: f64 = row.try_get("soft_breaking_score")
            .map_err(|e| Error::generic(&format!("Failed to get soft_breaking_score: {}", e)))?;
        let llm_analysis: serde_json::Value = row.try_get("llm_analysis").unwrap_or_default();
        let before_state: serde_json::Value = row.try_get("before_semantic_state").unwrap_or_default();
        let after_state: serde_json::Value = row.try_get("after_semantic_state").unwrap_or_default();
        let details: serde_json::Value = row.try_get("details").unwrap_or_default();
        let related_id: Option<Uuid> = row.try_get("related_drift_incident_id").ok();
        let contract_diff_id: Option<String> = row.try_get("contract_diff_id").ok();
        let external_ticket_id: Option<String> = row.try_get("external_ticket_id").ok();
        let external_ticket_url: Option<String> = row.try_get("external_ticket_url").ok();
        let detected_at: DateTime<Utc> = row.try_get("detected_at")
            .map_err(|e| Error::generic(&format!("Failed to get detected_at: {}", e)))?;
        let created_at: DateTime<Utc> = row.try_get("created_at")
            .map_err(|e| Error::generic(&format!("Failed to get created_at: {}", e)))?;
        let acknowledged_at: Option<DateTime<Utc>> = row.try_get("acknowledged_at").ok();
        let resolved_at: Option<DateTime<Utc>> = row.try_get("resolved_at").ok();
        let closed_at: Option<DateTime<Utc>> = row.try_get("closed_at").ok();
        let updated_at: DateTime<Utc> = row.try_get("updated_at")
            .map_err(|e| Error::generic(&format!("Failed to get updated_at: {}", e)))?;

        let change_type = match change_type_str.as_str() {
            "description_change" => SemanticChangeType::DescriptionChange,
            "enum_narrowing" => SemanticChangeType::EnumNarrowing,
            "nullable_change" => SemanticChangeType::NullableChange,
            "error_code_removed" => SemanticChangeType::ErrorCodeRemoved,
            "semantic_constraint_change" => SemanticChangeType::SemanticConstraintChange,
            "meaning_shift" => SemanticChangeType::MeaningShift,
            "soft_breaking_change" => SemanticChangeType::SoftBreakingChange,
            _ => continue,
        };

        let severity = match severity_str.as_str() {
            "low" => IncidentSeverity::Low,
            "medium" => IncidentSeverity::Medium,
            "high" => IncidentSeverity::High,
            "critical" => IncidentSeverity::Critical,
            _ => continue,
        };

        let status = match status_str.as_str() {
            "open" => IncidentStatus::Open,
            "acknowledged" => IncidentStatus::Acknowledged,
            "resolved" => IncidentStatus::Resolved,
            "closed" => IncidentStatus::Closed,
            _ => continue,
        };

        incidents.push(SemanticIncident {
            id: id.to_string(),
            workspace_id: workspace_id.map(|u| u.to_string()),
            endpoint,
            method,
            semantic_change_type: change_type,
            severity,
            status,
            semantic_confidence,
            soft_breaking_score,
            llm_analysis,
            before_semantic_state: before_state,
            after_semantic_state: after_state,
            details,
            related_drift_incident_id: related_id.map(|u| u.to_string()),
            contract_diff_id,
            external_ticket_id,
            external_ticket_url,
            detected_at: detected_at.timestamp(),
            created_at: created_at.timestamp(),
            acknowledged_at: acknowledged_at.map(|dt| dt.timestamp()),
            resolved_at: resolved_at.map(|dt| dt.timestamp()),
            closed_at: closed_at.map(|dt| dt.timestamp()),
            updated_at: updated_at.timestamp(),
        });
    }

    Ok(incidents)
}

/// Query threat assessments from database
async fn query_threat_assessments(
    pool: &PgPool,
    workspace_id: Option<&str>,
    service_id: Option<&str>,
) -> Result<Vec<ThreatAssessment>> {
    use sqlx::Row;

    let mut query = String::from(
        "SELECT id, workspace_id, service_id, service_name, endpoint, method,
         aggregation_level, threat_level, threat_score, threat_categories,
         findings, remediation_suggestions, assessed_at
         FROM contract_threat_assessments
         WHERE 1=1",
    );

    let mut bind_index = 1;
    let mut params: Vec<Option<Uuid>> = Vec::new();
    let mut string_params: Vec<Option<String>> = Vec::new();

    if let Some(ws_id) = workspace_id {
        query.push_str(&format!(" AND workspace_id = ${}", bind_index));
        bind_index += 1;
        params.push(Uuid::parse_str(ws_id).ok());
    }

    if let Some(svc_id) = service_id {
        query.push_str(&format!(" AND service_id = ${}", bind_index));
        bind_index += 1;
        string_params.push(Some(svc_id.to_string()));
    }

    query.push_str(" ORDER BY assessed_at DESC LIMIT 50");

    // Build query with parameters
    let mut query_builder = sqlx::query(&query);
    for param in params {
        query_builder = query_builder.bind(param);
    }
    for param in string_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder
        .fetch_all(pool)
        .await
        .map_err(|e| Error::generic(&format!("Failed to query threat assessments: {}", e)))?;

    let mut assessments = Vec::new();
    for row in rows {
        match map_row_to_threat_assessment(&row) {
            Ok(assessment) => assessments.push(assessment),
            Err(e) => {
                warn!("Failed to map threat assessment row: {}", e);
                continue;
            }
        }
    }

    Ok(assessments)
}

/// Map database row to ThreatAssessment
fn map_row_to_threat_assessment(row: &sqlx::postgres::PgRow) -> Result<ThreatAssessment> {
    use sqlx::Row;
    use mockforge_core::contract_drift::threat_modeling::{ThreatCategory, ThreatFinding, RemediationSuggestion};

    let workspace_id: Option<Uuid> = row.try_get("workspace_id").ok();
    let service_id: Option<String> = row.try_get("service_id").ok();
    let service_name: Option<String> = row.try_get("service_name").ok();
    let endpoint: Option<String> = row.try_get("endpoint").ok();
    let method: Option<String> = row.try_get("method").ok();
    let aggregation_level_str: String = row.try_get("aggregation_level")
        .map_err(|e| Error::generic(&format!("Failed to get aggregation_level: {}", e)))?;
    let threat_level_str: String = row.try_get("threat_level")
        .map_err(|e| Error::generic(&format!("Failed to get threat_level: {}", e)))?;
    let threat_score: f64 = row.try_get("threat_score")
        .map_err(|e| Error::generic(&format!("Failed to get threat_score: {}", e)))?;
    let threat_categories_json: serde_json::Value = row.try_get("threat_categories").unwrap_or_default();
    let findings_json: serde_json::Value = row.try_get("findings").unwrap_or_default();
    let remediations_json: serde_json::Value = row.try_get("remediation_suggestions").unwrap_or_default();
    let assessed_at: DateTime<Utc> = row.try_get("assessed_at")
        .map_err(|e| Error::generic(&format!("Failed to get assessed_at: {}", e)))?;

    let aggregation_level = match aggregation_level_str.as_str() {
        "workspace" => mockforge_core::contract_drift::threat_modeling::AggregationLevel::Workspace,
        "service" => mockforge_core::contract_drift::threat_modeling::AggregationLevel::Service,
        "endpoint" => mockforge_core::contract_drift::threat_modeling::AggregationLevel::Endpoint,
        _ => return Err(Error::generic(&format!("Invalid aggregation_level: {}", aggregation_level_str))),
    };

    let threat_level = match threat_level_str.as_str() {
        "low" => ThreatLevel::Low,
        "medium" => ThreatLevel::Medium,
        "high" => ThreatLevel::High,
        "critical" => ThreatLevel::Critical,
        _ => return Err(Error::generic(&format!("Invalid threat_level: {}", threat_level_str))),
    };

    let threat_categories: Vec<ThreatCategory> = serde_json::from_value(threat_categories_json).unwrap_or_default();
    let findings: Vec<ThreatFinding> = serde_json::from_value(findings_json).unwrap_or_default();
    let remediation_suggestions: Vec<RemediationSuggestion> = serde_json::from_value(remediations_json).unwrap_or_default();

    Ok(ThreatAssessment {
        workspace_id: workspace_id.map(|u| u.to_string()),
        service_id,
        service_name,
        endpoint,
        method,
        aggregation_level,
        threat_level,
        threat_score,
        threat_categories,
        findings,
        remediation_suggestions,
        assessed_at,
    })
}

/// Handle the governance status command
pub async fn handle_governance_status(
    workspace_id: Option<String>,
    service_id: Option<String>,
) -> Result<()> {
    info!("Checking contract governance health status");

    let ws_display = workspace_id.as_deref().unwrap_or("default");
    info!("Workspace: {}", ws_display);
    if let Some(svc_id) = &service_id {
        info!("Service: {}", svc_id);
    }

    // Query database for governance data
    if let Some(pool) = connect_database().await {
        // Query recent drift incidents (last 30 days)
        let drift_incidents = match query_drift_incidents(
            &pool,
            workspace_id.as_deref(),
            service_id.as_deref(),
            None,
            None,
            30,
        )
        .await
        {
            Ok(incidents) => {
                info!("Found {} drift incidents (last 30 days)", incidents.len());
                incidents
            }
            Err(e) => {
                warn!("Failed to query drift incidents: {}", e);
                Vec::new()
            }
        };

        // Query recent semantic incidents (last 30 days)
        let semantic_incidents = match query_semantic_incidents(
            &pool,
            workspace_id.as_deref(),
            30,
        )
        .await
        {
            Ok(incidents) => {
                info!("Found {} semantic incidents (last 30 days)", incidents.len());
                incidents
            }
            Err(e) => {
                warn!("Failed to query semantic incidents: {}", e);
                Vec::new()
            }
        };

        // Query active threat assessments
        let threat_assessments = match query_threat_assessments(
            &pool,
            workspace_id.as_deref(),
            service_id.as_deref(),
        )
        .await
        {
            Ok(assessments) => {
                info!("Found {} threat assessments", assessments.len());
                assessments
            }
            Err(e) => {
                warn!("Failed to query threat assessments: {}", e);
                Vec::new()
            }
        };

        // Display summary
        println!("\n=== Governance Health Summary ===");

        // Drift incidents summary
        let open_drift = drift_incidents.iter().filter(|i| matches!(i.status, IncidentStatus::Open)).count();
        let critical_drift = drift_incidents.iter().filter(|i| matches!(i.severity, IncidentSeverity::Critical)).count();
        let breaking_changes = drift_incidents.iter().filter(|i| matches!(i.incident_type, IncidentType::BreakingChange)).count();

        println!("\nDrift Incidents (last 30 days):");
        println!("  Total: {}", drift_incidents.len());
        println!("  Open: {}", open_drift);
        println!("  Critical: {}", critical_drift);
        println!("  Breaking Changes: {}", breaking_changes);

        // Semantic incidents summary
        let open_semantic = semantic_incidents.iter().filter(|i| matches!(i.status, IncidentStatus::Open)).count();
        let high_confidence = semantic_incidents.iter().filter(|i| i.semantic_confidence >= 0.8).count();
        let soft_breaking = semantic_incidents.iter().filter(|i| i.soft_breaking_score >= 0.65).count();

        println!("\nSemantic Incidents (last 30 days):");
        println!("  Total: {}", semantic_incidents.len());
        println!("  Open: {}", open_semantic);
        println!("  High Confidence (>=0.8): {}", high_confidence);
        println!("  Soft-Breaking (>=0.65): {}", soft_breaking);

        // Threat assessments summary
        let critical_threats = threat_assessments.iter().filter(|a| matches!(a.threat_level, ThreatLevel::Critical)).count();
        let high_threats = threat_assessments.iter().filter(|a| matches!(a.threat_level, ThreatLevel::High)).count();
        let total_findings: usize = threat_assessments.iter().map(|a| a.findings.len()).sum();

        println!("\nThreat Assessments:");
        println!("  Total: {}", threat_assessments.len());
        println!("  Critical: {}", critical_threats);
        println!("  High: {}", high_threats);
        println!("  Total Findings: {}", total_findings);

        // Overall health score (simple calculation)
        let total_issues = open_drift + open_semantic + critical_threats;
        let health_status = if total_issues == 0 {
            "Healthy"
        } else if total_issues <= 5 {
            "Moderate"
        } else if total_issues <= 15 {
            "Degraded"
        } else {
            "Critical"
        };

        println!("\nOverall Health Status: {}", health_status);
        println!("  Total Open Issues: {}", total_issues);

    } else {
        warn!("Database not available. Set DATABASE_URL environment variable to enable governance status queries.");
        info!("Workspace: {}", ws_display);
        if let Some(svc_id) = service_id {
            info!("Service: {}", svc_id);
        }
    }

    info!("Governance status check completed");
    Ok(())
}

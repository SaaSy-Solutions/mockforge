//! Workspace builder for creating complete workspaces from parsed commands
//!
//! This module provides functionality to build complete workspaces including:
//! - Workspace creation and registration
//! - OpenAPI spec generation and application
//! - Persona creation with relationships
//! - Behavioral scenario generation
//! - Reality continuum configuration
//! - Drift budget configuration

use crate::contract_drift::{DriftBudget, DriftBudgetConfig};
use crate::multi_tenant::MultiTenantWorkspaceRegistry;
use crate::openapi::OpenApiSpec;
use crate::reality_continuum::config::{
    ContinuumConfig, ContinuumRule, MergeStrategy, TransitionMode,
};
use crate::scenarios::types::{ScenarioDefinition, ScenarioStep};
use crate::voice::command_parser::{EndpointRequirement, ModelRequirement, ParsedCommand};
use crate::voice::command_parser::{
    ParsedDriftBudget, ParsedRealityContinuum, ParsedWorkspaceCreation,
};
use crate::voice::spec_generator::VoiceSpecGenerator;
use crate::Result;
use crate::Workspace;
use mockforge_data::{Domain, PersonaProfile};
use std::collections::HashMap;

/// Result of building a workspace
#[derive(Debug, Clone)]
pub struct BuiltWorkspace {
    /// Workspace ID
    pub workspace_id: String,
    /// Workspace name
    pub name: String,
    /// Generated OpenAPI spec (if any)
    pub openapi_spec: Option<OpenApiSpec>,
    /// Created personas
    pub personas: Vec<PersonaProfile>,
    /// Created scenarios
    pub scenarios: Vec<ScenarioDefinition>,
    /// Reality continuum config (if any)
    pub reality_continuum: Option<ContinuumConfig>,
    /// Drift budget config (if any)
    pub drift_budget: Option<DriftBudgetConfig>,
    /// Creation log
    pub creation_log: Vec<String>,
}

/// Workspace builder that creates complete workspaces from parsed commands
pub struct WorkspaceBuilder {
    /// Creation log for tracking what was created
    creation_log: Vec<String>,
}

impl WorkspaceBuilder {
    /// Create a new workspace builder
    pub fn new() -> Self {
        Self {
            creation_log: Vec::new(),
        }
    }

    /// Build a complete workspace from parsed creation command
    ///
    /// This method:
    /// 1. Validates requirements
    /// 2. Creates the workspace
    /// 3. Generates and applies OpenAPI spec
    /// 4. Creates personas with relationships
    /// 5. Creates behavioral scenarios
    /// 6. Applies reality continuum config
    /// 7. Applies drift budget config
    pub async fn build_workspace(
        &mut self,
        registry: &mut MultiTenantWorkspaceRegistry,
        parsed: &ParsedWorkspaceCreation,
    ) -> Result<BuiltWorkspace> {
        self.creation_log.clear();

        // Validate requirements
        self.validate_requirements(parsed)?;

        // Generate workspace ID from name (sanitize)
        let workspace_id = Self::sanitize_workspace_id(&parsed.workspace_name);

        // Check if workspace already exists
        if registry.get_workspace(&workspace_id).is_ok() {
            // Suggest alternative names
            let suggestions = self.suggest_workspace_names(&workspace_id, registry)?;
            return Err(crate::Error::generic(format!(
                "Workspace '{}' already exists. Suggested alternatives: {}",
                workspace_id,
                suggestions.join(", ")
            )));
        }

        self.log(&format!("Creating workspace: {}", workspace_id));

        // Create workspace
        let mut workspace = Workspace::new(parsed.workspace_name.clone());
        workspace.description = Some(parsed.workspace_description.clone());

        // Register workspace
        registry.register_workspace(workspace_id.clone(), workspace)?;
        self.log(&format!("✓ Workspace '{}' created", workspace_id));

        // Generate OpenAPI spec from entities
        let openapi_spec = self.generate_openapi_spec(parsed).await?;

        // Create personas
        let personas = self.create_personas(parsed)?;
        self.log(&format!("✓ Created {} personas", personas.len()));

        // Create scenarios
        let scenarios = self.create_scenarios(parsed, &personas)?;
        self.log(&format!("✓ Created {} scenarios", scenarios.len()));

        // Apply reality continuum config
        let reality_continuum = if let Some(ref parsed_rc) = parsed.reality_continuum {
            Some(self.apply_reality_config(parsed_rc)?)
        } else {
            None
        };
        if reality_continuum.is_some() {
            self.log("✓ Reality continuum configured");
        }

        // Apply drift budget config
        let drift_budget = if let Some(ref parsed_db) = parsed.drift_budget {
            Some(self.apply_drift_budget(parsed_db)?)
        } else {
            None
        };
        if drift_budget.is_some() {
            self.log("✓ Drift budget configured");
        }

        Ok(BuiltWorkspace {
            workspace_id,
            name: parsed.workspace_name.clone(),
            openapi_spec,
            personas,
            scenarios,
            reality_continuum,
            drift_budget,
            creation_log: self.creation_log.clone(),
        })
    }

    /// Generate OpenAPI spec from entities
    async fn generate_openapi_spec(
        &mut self,
        parsed: &ParsedWorkspaceCreation,
    ) -> Result<Option<OpenApiSpec>> {
        if parsed.entities.is_empty() {
            return Ok(None);
        }

        self.log("Generating OpenAPI specification...");

        // Convert entities to ParsedCommand format
        let mut endpoints = Vec::new();
        let mut models = Vec::new();

        for entity in &parsed.entities {
            // Add endpoints
            for endpoint in &entity.endpoints {
                endpoints.push(EndpointRequirement {
                    path: endpoint.path.clone(),
                    method: endpoint.method.clone(),
                    description: endpoint.description.clone(),
                    request_body: None,
                    response: None,
                });
            }

            // Add model
            if !entity.fields.is_empty() {
                models.push(ModelRequirement {
                    name: entity.name.clone(),
                    fields: entity.fields.clone(),
                });
            }
        }

        // Create ParsedCommand
        let parsed_command = ParsedCommand {
            api_type: "workspace".to_string(),
            title: parsed.workspace_name.clone(),
            description: parsed.workspace_description.clone(),
            endpoints,
            models,
            relationships: vec![],
            sample_counts: HashMap::new(),
            flows: vec![],
        };

        // Generate spec
        let spec_generator = VoiceSpecGenerator::new();
        let spec = spec_generator.generate_spec(&parsed_command).await?;

        self.log(&format!(
            "✓ Generated OpenAPI spec with {} endpoints",
            spec.all_paths_and_operations().len()
        ));

        Ok(Some(spec))
    }

    /// Create personas from requirements
    fn create_personas(&mut self, parsed: &ParsedWorkspaceCreation) -> Result<Vec<PersonaProfile>> {
        let mut personas = Vec::new();

        // Determine domain from workspace description or default to General
        let domain = Self::infer_domain(&parsed.workspace_description);

        for persona_req in &parsed.personas {
            // Generate persona ID
            let persona_id = format!("persona:{}", Self::sanitize_id(&persona_req.name));

            // Create persona
            let mut persona =
                PersonaProfile::with_traits(persona_id.clone(), domain, persona_req.traits.clone());

            // Set backstory from description
            persona.backstory = Some(persona_req.description.clone());

            // Build relationships map
            let mut relationships = HashMap::new();
            for rel in &persona_req.relationships {
                let rel_key = format!("{}:{}", rel.r#type, rel.target_entity);
                relationships
                    .entry(rel_key)
                    .or_insert_with(Vec::new)
                    .push(format!("entity:{}", Self::sanitize_id(&rel.target_entity)));
            }
            persona.relationships = relationships;

            personas.push(persona);
        }

        Ok(personas)
    }

    /// Create scenarios from requirements
    fn create_scenarios(
        &mut self,
        parsed: &ParsedWorkspaceCreation,
        personas: &[PersonaProfile],
    ) -> Result<Vec<ScenarioDefinition>> {
        let mut scenarios = Vec::new();

        for scenario_req in &parsed.scenarios {
            // Generate scenario ID
            let scenario_id = Self::sanitize_id(&scenario_req.name);

            // Create scenario
            let mut scenario =
                ScenarioDefinition::new(scenario_id.clone(), scenario_req.name.clone());
            scenario.description = Some(scenario_req.description.clone());

            // Add tags based on type
            scenario.tags = vec![scenario_req.r#type.clone()];

            // Convert steps
            for (idx, step_req) in scenario_req.steps.iter().enumerate() {
                // Parse endpoint (e.g., "POST /api/orders")
                let (method, path) = Self::parse_endpoint(&step_req.endpoint)?;

                let mut step = ScenarioStep::new(
                    format!("step-{}", idx + 1),
                    step_req.description.clone(),
                    method,
                    path,
                )
                .expect_status(if scenario_req.r#type == "failure" {
                    400
                } else {
                    200
                });

                // Add delay for slow_path scenarios
                if scenario_req.r#type == "slow_path" {
                    step.delay_ms = Some(2000); // 2 second delay for slow paths
                }

                scenario = scenario.add_step(step);
            }

            scenarios.push(scenario);
        }

        Ok(scenarios)
    }

    /// Apply reality continuum configuration
    fn apply_reality_config(&mut self, parsed: &ParsedRealityContinuum) -> Result<ContinuumConfig> {
        let mut config = ContinuumConfig::default();

        if parsed.enabled {
            config = config.enable();
        }

        config = config.with_default_ratio(parsed.default_ratio);

        // Set transition mode
        let transition_mode = match parsed.transition_mode.as_str() {
            "time_based" => TransitionMode::TimeBased,
            "scheduled" => TransitionMode::Scheduled,
            _ => TransitionMode::Manual,
        };
        config = config.with_transition_mode(transition_mode);

        // Set merge strategy
        let merge_strategy = match parsed.merge_strategy.as_str() {
            "weighted" => MergeStrategy::Weighted,
            "body_blend" => MergeStrategy::BodyBlend,
            _ => MergeStrategy::FieldLevel,
        };
        config = config.with_merge_strategy(merge_strategy);

        // Add route rules
        for rule in &parsed.route_rules {
            let continuum_rule = ContinuumRule::new(rule.pattern.clone(), rule.ratio);
            config = config.add_route(continuum_rule);
        }

        Ok(config)
    }

    /// Apply drift budget configuration
    fn apply_drift_budget(&mut self, parsed: &ParsedDriftBudget) -> Result<DriftBudgetConfig> {
        let mut config = DriftBudgetConfig::default();

        if !parsed.enabled {
            config.enabled = false;
        }

        // Set default budget based on strictness
        let default_budget = DriftBudget {
            max_breaking_changes: parsed.max_breaking_changes,
            max_non_breaking_changes: parsed.max_non_breaking_changes,
            max_field_churn_percent: parsed.max_field_churn_percent,
            time_window_days: parsed.time_window_days,
            severity_threshold: crate::ai_contract_diff::MismatchSeverity::High,
            enabled: parsed.enabled,
        };
        config.default_budget = Some(default_budget);

        // Add per-service budgets
        for (service_name, service_budget) in &parsed.per_service_budgets {
            let budget = DriftBudget {
                max_breaking_changes: service_budget.max_breaking_changes,
                max_non_breaking_changes: service_budget.max_non_breaking_changes,
                max_field_churn_percent: None,
                time_window_days: None,
                severity_threshold: crate::ai_contract_diff::MismatchSeverity::High,
                enabled: parsed.enabled,
            };
            config.per_service_budgets.insert(service_name.clone(), budget);
        }

        Ok(config)
    }

    /// Sanitize workspace ID from name
    fn sanitize_workspace_id(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    /// Sanitize ID string
    fn sanitize_id(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    /// Infer domain from description
    fn infer_domain(description: &str) -> Domain {
        let desc_lower = description.to_lowercase();
        if desc_lower.contains("e-commerce")
            || desc_lower.contains("ecommerce")
            || desc_lower.contains("shop")
        {
            Domain::Ecommerce
        } else if desc_lower.contains("bank")
            || desc_lower.contains("finance")
            || desc_lower.contains("payment")
        {
            Domain::Finance
        } else if desc_lower.contains("health") || desc_lower.contains("medical") {
            Domain::Healthcare
        } else if desc_lower.contains("iot") || desc_lower.contains("device") {
            Domain::Iot
        } else {
            Domain::General
        }
    }

    /// Parse endpoint string (e.g., "POST /api/orders") into method and path
    fn parse_endpoint(endpoint: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = endpoint.trim().splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(crate::Error::generic(format!(
                "Invalid endpoint format: {}. Expected 'METHOD /path'",
                endpoint
            )));
        }

        let method = parts[0].to_uppercase();
        let path = parts[1].to_string();

        Ok((method, path))
    }

    /// Validate workspace creation requirements
    ///
    /// Ensures:
    /// - At least 2-3 endpoints per entity
    /// - 2-3 personas with relationships
    /// - 2-3 behavioral scenarios
    fn validate_requirements(&self, parsed: &ParsedWorkspaceCreation) -> Result<()> {
        // Validate endpoints per entity (at least 2 per entity)
        for entity in &parsed.entities {
            if entity.endpoints.len() < 2 {
                return Err(crate::Error::generic(format!(
                    "Entity '{}' must have at least 2 endpoints. Found {}.",
                    entity.name,
                    entity.endpoints.len()
                )));
            }
        }

        // Validate personas (at least 2)
        if parsed.personas.len() < 2 {
            return Err(crate::Error::generic(format!(
                "Workspace must have at least 2 personas. Found {}.",
                parsed.personas.len()
            )));
        }

        // Validate persona relationships (each persona should have at least one relationship)
        for persona in &parsed.personas {
            if persona.relationships.is_empty() {
                return Err(crate::Error::generic(format!(
                    "Persona '{}' must have at least one relationship.",
                    persona.name
                )));
            }
        }

        // Validate scenarios (at least 2)
        if parsed.scenarios.len() < 2 {
            return Err(crate::Error::generic(format!(
                "Workspace must have at least 2 behavioral scenarios. Found {}.",
                parsed.scenarios.len()
            )));
        }

        // Validate scenario types (should have at least one of each type: happy_path, failure, slow_path)
        let scenario_types: Vec<&str> =
            parsed.scenarios.iter().map(|s| s.r#type.as_str()).collect();
        let has_happy = scenario_types.contains(&"happy_path");
        let has_failure = scenario_types.contains(&"failure");
        let has_slow = scenario_types.contains(&"slow_path");

        if !has_happy && !has_failure && !has_slow {
            return Err(crate::Error::generic(
                "Workspace must have at least one scenario of type: happy_path, failure, or slow_path".to_string(),
            ));
        }

        Ok(())
    }

    /// Suggest alternative workspace names if the requested name already exists
    fn suggest_workspace_names(
        &self,
        base_name: &str,
        registry: &MultiTenantWorkspaceRegistry,
    ) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();
        let mut counter = 1;

        // Try variations: name-1, name-2, etc.
        while suggestions.len() < 3 && counter < 10 {
            let candidate = format!("{}-{}", base_name, counter);
            if registry.get_workspace(&candidate).is_err() {
                suggestions.push(candidate);
            }
            counter += 1;
        }

        // If we still don't have enough, try with timestamp
        if suggestions.len() < 3 {
            let timestamp = chrono::Utc::now().format("%Y%m%d");
            let candidate = format!("{}-{}", base_name, timestamp);
            if registry.get_workspace(&candidate).is_err() {
                suggestions.push(candidate);
            }
        }

        Ok(suggestions)
    }

    /// Log a message
    fn log(&mut self, message: &str) {
        self.creation_log.push(message.to_string());
    }
}

impl Default for WorkspaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

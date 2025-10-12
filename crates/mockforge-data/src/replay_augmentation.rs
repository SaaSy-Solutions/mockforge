//! LLM-powered replay augmentation for WebSocket and GraphQL subscriptions
//!
//! This module enables AI-driven event stream generation for real-time protocols,
//! allowing users to define high-level scenarios that generate realistic event sequences.

use crate::rag::{RagConfig, RagEngine};
use mockforge_core::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::interval;

/// Replay augmentation mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayMode {
    /// Static replay from pre-recorded events
    Static,
    /// LLM-augmented replay with scenario-based generation
    Augmented,
    /// Fully generated event stream from narrative description
    Generated,
}

impl Default for ReplayMode {
    fn default() -> Self {
        Self::Static
    }
}

/// Event generation strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStrategy {
    /// Time-based event generation
    TimeBased,
    /// Count-based event generation
    CountBased,
    /// Condition-based event generation
    ConditionalBased,
}

/// Replay augmentation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayAugmentationConfig {
    /// Replay mode
    pub mode: ReplayMode,
    /// Narrative description of the scenario
    pub narrative: Option<String>,
    /// Event type/name
    pub event_type: String,
    /// Event schema (optional JSON schema)
    pub event_schema: Option<Value>,
    /// Event generation strategy
    pub strategy: EventStrategy,
    /// Duration to replay (for time-based)
    pub duration_secs: Option<u64>,
    /// Number of events to generate (for count-based)
    pub event_count: Option<usize>,
    /// Event rate (events per second)
    pub event_rate: Option<f64>,
    /// Conditions for event generation
    pub conditions: Vec<EventCondition>,
    /// RAG configuration for LLM
    pub rag_config: Option<RagConfig>,
    /// Enable progressive scenario evolution
    pub progressive_evolution: bool,
}

impl Default for ReplayAugmentationConfig {
    fn default() -> Self {
        Self {
            mode: ReplayMode::Static,
            narrative: None,
            event_type: "event".to_string(),
            event_schema: None,
            strategy: EventStrategy::CountBased,
            duration_secs: None,
            event_count: Some(10),
            event_rate: Some(1.0),
            conditions: Vec::new(),
            rag_config: None,
            progressive_evolution: true,
        }
    }
}

/// Event generation condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCondition {
    /// Condition name/description
    pub name: String,
    /// Condition expression (simplified)
    pub expression: String,
    /// Action to take when condition is met
    pub action: ConditionAction,
}

/// Condition action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionAction {
    /// Generate a new event
    GenerateEvent,
    /// Stop event generation
    Stop,
    /// Change event rate
    ChangeRate(u64), // events per second
    /// Transition to new scenario
    TransitionScenario(String),
}

/// Generated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedEvent {
    /// Event type
    pub event_type: String,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event data
    pub data: Value,
    /// Sequence number
    pub sequence: usize,
    /// Event metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl GeneratedEvent {
    /// Create a new generated event
    pub fn new(event_type: String, data: Value, sequence: usize) -> Self {
        Self {
            event_type,
            timestamp: chrono::Utc::now(),
            data,
            sequence,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| Error::generic(format!("Failed to serialize event: {}", e)))
    }
}

/// Replay augmentation engine
pub struct ReplayAugmentationEngine {
    /// Configuration
    config: ReplayAugmentationConfig,
    /// RAG engine for LLM-based generation
    rag_engine: Option<RagEngine>,
    /// Event sequence counter
    sequence: usize,
    /// Current scenario state
    scenario_state: ScenarioState,
}

/// Scenario state tracking
#[derive(Debug, Clone)]
struct ScenarioState {
    /// Current timestamp in scenario
    current_time: std::time::Instant,
    /// Events generated so far
    events_generated: usize,
    /// Last event data (for progressive evolution)
    last_event: Option<Value>,
    /// Scenario context
    context: Vec<String>,
}

impl Default for ScenarioState {
    fn default() -> Self {
        Self {
            current_time: std::time::Instant::now(),
            events_generated: 0,
            last_event: None,
            context: Vec::new(),
        }
    }
}

impl ReplayAugmentationEngine {
    /// Create a new replay augmentation engine
    pub fn new(config: ReplayAugmentationConfig) -> Result<Self> {
        Self::validate_config(&config)?;

        let rag_engine = if config.mode != ReplayMode::Static {
            let rag_config = config.rag_config.clone().unwrap_or_default();
            Some(RagEngine::new(rag_config))
        } else {
            None
        };

        Ok(Self {
            config,
            rag_engine,
            sequence: 0,
            scenario_state: ScenarioState::default(),
        })
    }

    /// Validate configuration
    fn validate_config(config: &ReplayAugmentationConfig) -> Result<()> {
        if config.mode != ReplayMode::Static && config.narrative.is_none() {
            return Err(Error::generic(
                "Narrative is required for augmented or generated replay modes",
            ));
        }

        match config.strategy {
            EventStrategy::TimeBased => {
                if config.duration_secs.is_none() {
                    return Err(Error::generic(
                        "Duration must be specified for time-based strategy",
                    ));
                }
            }
            EventStrategy::CountBased => {
                if config.event_count.is_none() {
                    return Err(Error::generic(
                        "Event count must be specified for count-based strategy",
                    ));
                }
            }
            EventStrategy::ConditionalBased => {
                if config.conditions.is_empty() {
                    return Err(Error::generic(
                        "Conditions must be specified for conditional-based strategy",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Generate event stream based on configuration
    pub async fn generate_stream(&mut self) -> Result<Vec<GeneratedEvent>> {
        match self.config.strategy {
            EventStrategy::CountBased => self.generate_count_based().await,
            EventStrategy::TimeBased => self.generate_time_based().await,
            EventStrategy::ConditionalBased => self.generate_conditional_based().await,
        }
    }

    /// Generate events based on count
    async fn generate_count_based(&mut self) -> Result<Vec<GeneratedEvent>> {
        let count = self.config.event_count.unwrap_or(10);
        let mut events = Vec::with_capacity(count);

        for i in 0..count {
            let event = self.generate_single_event(i).await?;
            events.push(event);

            // Add delay between events if rate is specified
            if let Some(rate) = self.config.event_rate {
                if rate > 0.0 {
                    let delay_ms = (1000.0 / rate) as u64;
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Ok(events)
    }

    /// Generate events based on time duration
    async fn generate_time_based(&mut self) -> Result<Vec<GeneratedEvent>> {
        let duration = Duration::from_secs(self.config.duration_secs.unwrap_or(60));
        let rate = self.config.event_rate.unwrap_or(1.0);
        let interval_ms = (1000.0 / rate) as u64;

        let mut events = Vec::new();
        let mut ticker = interval(Duration::from_millis(interval_ms));
        let start = std::time::Instant::now();

        let mut index = 0;
        while start.elapsed() < duration {
            ticker.tick().await;
            let event = self.generate_single_event(index).await?;
            events.push(event);
            index += 1;
        }

        Ok(events)
    }

    /// Generate events based on conditions
    async fn generate_conditional_based(&mut self) -> Result<Vec<GeneratedEvent>> {
        let mut events = Vec::new();
        let mut index = 0;
        let max_events = 1000; // Safety limit

        while index < max_events {
            // Check conditions
            let mut should_continue = true;
            let conditions = self.config.conditions.clone(); // Clone to avoid borrow issues

            for condition in &conditions {
                if self.evaluate_condition(condition, &events) {
                    match &condition.action {
                        ConditionAction::GenerateEvent => {
                            let event = self.generate_single_event(index).await?;
                            events.push(event);
                            index += 1;
                        }
                        ConditionAction::Stop => {
                            should_continue = false;
                            break;
                        }
                        ConditionAction::ChangeRate(_rate) => {
                            // Update rate (would require mutable config)
                        }
                        ConditionAction::TransitionScenario(_scenario) => {
                            // Transition to new scenario
                            self.scenario_state.context.clear();
                        }
                    }
                }
            }

            if !should_continue {
                break;
            }

            // Prevent infinite loop
            if events.is_empty() && index > 10 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(events)
    }

    /// Generate a single event
    async fn generate_single_event(&mut self, index: usize) -> Result<GeneratedEvent> {
        let data = match self.config.mode {
            ReplayMode::Static => self.generate_static_event(),
            ReplayMode::Augmented => self.generate_augmented_event(index).await?,
            ReplayMode::Generated => self.generate_llm_event(index).await?,
        };

        self.sequence += 1;
        self.scenario_state.events_generated += 1;
        self.scenario_state.last_event = Some(data.clone());

        Ok(GeneratedEvent::new(self.config.event_type.clone(), data, self.sequence))
    }

    /// Generate static event (fallback)
    fn generate_static_event(&self) -> Value {
        if let Some(schema) = &self.config.event_schema {
            schema.clone()
        } else {
            serde_json::json!({
                "type": self.config.event_type,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
        }
    }

    /// Generate augmented event (base + LLM enhancement)
    async fn generate_augmented_event(&mut self, index: usize) -> Result<Value> {
        let mut base_event = self.generate_static_event();

        if let Some(rag_engine) = &self.rag_engine {
            let narrative = self.config.narrative.as_ref().unwrap();
            let prompt = self.build_augmentation_prompt(narrative, index)?;

            let enhancement = rag_engine.generate_text(&prompt).await?;
            let enhancement_json = self.parse_json_response(&enhancement)?;

            // Merge enhancement with base event
            if let (Some(base_obj), Some(enhancement_obj)) =
                (base_event.as_object_mut(), enhancement_json.as_object())
            {
                for (key, value) in enhancement_obj {
                    base_obj.insert(key.clone(), value.clone());
                }
            } else {
                base_event = enhancement_json;
            }
        }

        Ok(base_event)
    }

    /// Generate fully LLM-generated event
    async fn generate_llm_event(&mut self, index: usize) -> Result<Value> {
        let rag_engine = self
            .rag_engine
            .as_ref()
            .ok_or_else(|| Error::generic("RAG engine not initialized for generated mode"))?;

        let narrative = self.config.narrative.as_ref().unwrap();
        let prompt = self.build_generation_prompt(narrative, index)?;

        let response = rag_engine.generate_text(&prompt).await?;
        self.parse_json_response(&response)
    }

    /// Build augmentation prompt
    fn build_augmentation_prompt(&self, narrative: &str, index: usize) -> Result<String> {
        let mut prompt = format!(
            "Enhance this event data based on the following scenario:\n\n{}\n\n",
            narrative
        );

        prompt.push_str(&format!("Event #{} (out of ongoing stream)\n\n", index + 1));

        if let Some(last_event) = &self.scenario_state.last_event {
            prompt.push_str(&format!(
                "Previous event:\n{}\n\n",
                serde_json::to_string_pretty(last_event).unwrap_or_default()
            ));
        }

        if self.config.progressive_evolution {
            prompt.push_str("Progressively evolve the scenario with each event.\n");
        }

        if let Some(schema) = &self.config.event_schema {
            prompt.push_str(&format!(
                "Conform to this schema:\n{}\n\n",
                serde_json::to_string_pretty(schema).unwrap_or_default()
            ));
        }

        prompt.push_str("Return valid JSON only for the enhanced event data.");

        Ok(prompt)
    }

    /// Build generation prompt
    fn build_generation_prompt(&self, narrative: &str, index: usize) -> Result<String> {
        let mut prompt =
            format!("Generate realistic event data for this scenario:\n\n{}\n\n", narrative);

        prompt.push_str(&format!("Event type: {}\n", self.config.event_type));
        prompt.push_str(&format!("Event #{}\n\n", index + 1));

        if let Some(last_event) = &self.scenario_state.last_event {
            prompt.push_str(&format!(
                "Previous event:\n{}\n\n",
                serde_json::to_string_pretty(last_event).unwrap_or_default()
            ));

            if self.config.progressive_evolution {
                prompt.push_str("Naturally evolve from the previous event.\n");
            }
        }

        if let Some(schema) = &self.config.event_schema {
            prompt.push_str(&format!(
                "Conform to this schema:\n{}\n\n",
                serde_json::to_string_pretty(schema).unwrap_or_default()
            ));
        }

        prompt.push_str("Return valid JSON only.");

        Ok(prompt)
    }

    /// Parse JSON response from LLM
    fn parse_json_response(&self, response: &str) -> Result<Value> {
        let trimmed = response.trim();

        // Try to extract from markdown code blocks
        let json_str = if trimmed.starts_with("```json") {
            trimmed
                .strip_prefix("```json")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(trimmed)
                .trim()
        } else if trimmed.starts_with("```") {
            trimmed
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(trimmed)
                .trim()
        } else {
            trimmed
        };

        // Parse JSON
        serde_json::from_str(json_str)
            .map_err(|e| Error::generic(format!("Failed to parse LLM response as JSON: {}", e)))
    }

    /// Evaluate condition (simplified)
    fn evaluate_condition(&self, _condition: &EventCondition, events: &[GeneratedEvent]) -> bool {
        // Simplified condition evaluation
        // In a real implementation, this would parse and evaluate the expression
        events.len() < 100 // Just a placeholder
    }

    /// Reset the engine state
    pub fn reset(&mut self) {
        self.sequence = 0;
        self.scenario_state = ScenarioState::default();
    }

    /// Get current sequence number
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    /// Get events generated count
    pub fn events_generated(&self) -> usize {
        self.scenario_state.events_generated
    }
}

/// Pre-defined scenario templates
pub mod scenarios {
    use super::*;

    /// Stock market simulation scenario
    pub fn stock_market_scenario() -> ReplayAugmentationConfig {
        ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            narrative: Some(
                "Simulate 10 minutes of live market data with realistic price movements, \
                 volume changes, and occasional volatility spikes."
                    .to_string(),
            ),
            event_type: "market_tick".to_string(),
            event_schema: Some(serde_json::json!({
                "symbol": "string",
                "price": "number",
                "volume": "number",
                "timestamp": "string"
            })),
            strategy: EventStrategy::TimeBased,
            duration_secs: Some(600), // 10 minutes
            event_rate: Some(2.0),    // 2 events per second
            ..Default::default()
        }
    }

    /// Chat application scenario
    pub fn chat_messages_scenario() -> ReplayAugmentationConfig {
        ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            narrative: Some(
                "Simulate a group chat conversation between 3-5 users discussing a project, \
                 with natural message pacing and realistic content."
                    .to_string(),
            ),
            event_type: "chat_message".to_string(),
            event_schema: Some(serde_json::json!({
                "user_id": "string",
                "message": "string",
                "timestamp": "string"
            })),
            strategy: EventStrategy::CountBased,
            event_count: Some(50),
            event_rate: Some(0.5), // One message every 2 seconds
            ..Default::default()
        }
    }

    /// IoT sensor data scenario
    pub fn iot_sensor_scenario() -> ReplayAugmentationConfig {
        ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            narrative: Some(
                "Simulate IoT sensor readings from a smart building with temperature, \
                 humidity, and occupancy data showing daily patterns."
                    .to_string(),
            ),
            event_type: "sensor_reading".to_string(),
            event_schema: Some(serde_json::json!({
                "sensor_id": "string",
                "temperature": "number",
                "humidity": "number",
                "occupancy": "number",
                "timestamp": "string"
            })),
            strategy: EventStrategy::CountBased,
            event_count: Some(100),
            event_rate: Some(1.0),
            progressive_evolution: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_mode_default() {
        assert_eq!(ReplayMode::default(), ReplayMode::Static);
    }

    #[test]
    fn test_event_strategy_variants() {
        let time_based = EventStrategy::TimeBased;
        let count_based = EventStrategy::CountBased;
        let conditional = EventStrategy::ConditionalBased;

        assert!(matches!(time_based, EventStrategy::TimeBased));
        assert!(matches!(count_based, EventStrategy::CountBased));
        assert!(matches!(conditional, EventStrategy::ConditionalBased));
    }

    #[test]
    fn test_generated_event_creation() {
        let data = serde_json::json!({"test": "value"});
        let event = GeneratedEvent::new("test_event".to_string(), data, 1);

        assert_eq!(event.event_type, "test_event");
        assert_eq!(event.sequence, 1);
    }

    #[test]
    fn test_replay_config_validation_missing_narrative() {
        let config = ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            ..Default::default()
        };

        assert!(ReplayAugmentationEngine::validate_config(&config).is_err());
    }

    #[test]
    fn test_scenario_templates() {
        let stock_scenario = scenarios::stock_market_scenario();
        assert_eq!(stock_scenario.mode, ReplayMode::Generated);
        assert!(stock_scenario.narrative.is_some());

        let chat_scenario = scenarios::chat_messages_scenario();
        assert_eq!(chat_scenario.event_type, "chat_message");

        let iot_scenario = scenarios::iot_sensor_scenario();
        assert!(iot_scenario.progressive_evolution);
    }
}

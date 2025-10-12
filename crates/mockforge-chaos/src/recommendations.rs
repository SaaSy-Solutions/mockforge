//! AI-powered chaos recommendations
//!
//! Analyzes chaos engineering metrics and system behavior to generate
//! intelligent recommendations for improving resilience testing.

use crate::analytics::{ChaosImpact, MetricsBucket};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Recommendation category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationCategory {
    /// Latency testing recommendations
    Latency,
    /// Fault injection recommendations
    FaultInjection,
    /// Rate limiting recommendations
    RateLimit,
    /// Traffic shaping recommendations
    TrafficShaping,
    /// Circuit breaker recommendations
    CircuitBreaker,
    /// Bulkhead recommendations
    Bulkhead,
    /// Scenario recommendations
    Scenario,
    /// Coverage recommendations
    Coverage,
}

/// Recommendation severity/priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationSeverity {
    /// Informational
    Info,
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical - should be addressed immediately
    Critical,
}

/// Confidence level in the recommendation
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Confidence(f64);

impl Confidence {
    /// Create a new confidence value (0.0 - 1.0)
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get confidence value
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Check if confidence is high (>= 0.7)
    pub fn is_high(&self) -> bool {
        self.0 >= 0.7
    }

    /// Check if confidence is medium (0.4 - 0.7)
    pub fn is_medium(&self) -> bool {
        self.0 >= 0.4 && self.0 < 0.7
    }

    /// Check if confidence is low (< 0.4)
    pub fn is_low(&self) -> bool {
        self.0 < 0.4
    }
}

/// A chaos engineering recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Unique identifier
    pub id: String,
    /// Category
    pub category: RecommendationCategory,
    /// Severity/priority
    pub severity: RecommendationSeverity,
    /// Confidence level (0.0 - 1.0)
    pub confidence: Confidence,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Rationale - why this is recommended
    pub rationale: String,
    /// Suggested action
    pub action: String,
    /// Example configuration or command
    pub example: Option<String>,
    /// Affected endpoints/services
    pub affected_endpoints: Vec<String>,
    /// Related metrics
    pub metrics: HashMap<String, f64>,
    /// Generated timestamp
    pub generated_at: DateTime<Utc>,
    /// Expected impact score (0.0 - 1.0)
    pub expected_impact: f64,
}

impl Recommendation {
    /// Calculate overall recommendation score for prioritization
    pub fn score(&self) -> f64 {
        let severity_weight = match self.severity {
            RecommendationSeverity::Info => 0.2,
            RecommendationSeverity::Low => 0.4,
            RecommendationSeverity::Medium => 0.6,
            RecommendationSeverity::High => 0.8,
            RecommendationSeverity::Critical => 1.0,
        };

        // Weighted combination of severity, confidence, and expected impact
        (severity_weight * 0.4) + (self.confidence.value() * 0.3) + (self.expected_impact * 0.3)
    }
}

/// Pattern detected in chaos events
#[derive(Debug, Clone)]
struct ChaosPattern {
    /// Pattern type
    pattern_type: String,
    /// Frequency of occurrence
    frequency: f64,
    /// Affected components
    affected: Vec<String>,
    /// Severity
    severity: f64,
}

/// Weakness detected in system behavior
#[derive(Debug, Clone)]
struct SystemWeakness {
    /// Weakness type
    weakness_type: String,
    /// Affected endpoints
    endpoints: Vec<String>,
    /// Severity score (0.0 - 1.0)
    severity: f64,
    /// Evidence metrics
    evidence: HashMap<String, f64>,
}

/// AI-powered chaos recommendation engine
pub struct RecommendationEngine {
    /// Generated recommendations
    recommendations: Arc<RwLock<Vec<Recommendation>>>,
    /// Historical patterns
    patterns: Arc<RwLock<Vec<ChaosPattern>>>,
    /// Configuration
    config: EngineConfig,
}

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Minimum confidence threshold for recommendations
    pub min_confidence: f64,
    /// Maximum recommendations to generate
    pub max_recommendations: usize,
    /// Enable pattern learning
    pub enable_learning: bool,
    /// Analysis window (hours)
    pub analysis_window_hours: i64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_recommendations: 20,
            enable_learning: true,
            analysis_window_hours: 24,
        }
    }
}

impl RecommendationEngine {
    /// Create a new recommendation engine
    pub fn new() -> Self {
        Self::with_config(EngineConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: EngineConfig) -> Self {
        Self {
            recommendations: Arc::new(RwLock::new(Vec::new())),
            patterns: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Analyze metrics and generate recommendations
    pub fn analyze_and_recommend(
        &self,
        buckets: &[MetricsBucket],
        impact: &ChaosImpact,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Detect patterns
        let patterns = self.detect_patterns(buckets);

        // Detect weaknesses
        let weaknesses = self.detect_weaknesses(buckets, impact);

        // Generate recommendations from patterns
        recommendations.extend(self.recommendations_from_patterns(&patterns));

        // Generate recommendations from weaknesses
        recommendations.extend(self.recommendations_from_weaknesses(&weaknesses));

        // Generate coverage recommendations
        recommendations.extend(self.coverage_recommendations(buckets, impact));

        // Generate scenario recommendations
        recommendations.extend(self.scenario_recommendations(impact));

        // Score and filter by confidence
        let mut filtered: Vec<_> = recommendations
            .into_iter()
            .filter(|r| r.confidence.value() >= self.config.min_confidence)
            .collect();

        // Sort by score (highest first)
        filtered
            .sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to max recommendations
        filtered.truncate(self.config.max_recommendations);

        // Store recommendations
        {
            let mut recs = self.recommendations.write();
            *recs = filtered.clone();
        }

        // Update patterns if learning is enabled
        if self.config.enable_learning {
            let mut stored_patterns = self.patterns.write();
            *stored_patterns = patterns;
        }

        filtered
    }

    /// Detect patterns in chaos events
    fn detect_patterns(&self, buckets: &[MetricsBucket]) -> Vec<ChaosPattern> {
        let mut patterns = Vec::new();

        if buckets.is_empty() {
            return patterns;
        }

        // Pattern 1: Endpoints with consistently high latency
        let latency_endpoints = self.detect_latency_patterns(buckets);
        patterns.extend(latency_endpoints);

        // Pattern 2: Endpoints with high fault rates
        let fault_endpoints = self.detect_fault_patterns(buckets);
        patterns.extend(fault_endpoints);

        // Pattern 3: Rate limit violations
        let rate_limit_patterns = self.detect_rate_limit_patterns(buckets);
        patterns.extend(rate_limit_patterns);

        // Pattern 4: Time-based patterns
        let time_patterns = self.detect_time_patterns(buckets);
        patterns.extend(time_patterns);

        patterns
    }

    /// Detect latency patterns
    fn detect_latency_patterns(&self, buckets: &[MetricsBucket]) -> Vec<ChaosPattern> {
        let mut endpoint_latencies: HashMap<String, Vec<f64>> = HashMap::new();

        for bucket in buckets {
            for endpoint in bucket.affected_endpoints.keys() {
                endpoint_latencies
                    .entry(endpoint.clone())
                    .or_default()
                    .push(bucket.avg_latency_ms);
            }
        }

        endpoint_latencies
            .into_iter()
            .filter_map(|(endpoint, latencies)| {
                if latencies.is_empty() {
                    return None;
                }

                let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;

                // High latency pattern if average > 500ms
                if avg > 500.0 {
                    Some(ChaosPattern {
                        pattern_type: "high_latency".to_string(),
                        frequency: latencies.len() as f64 / buckets.len() as f64,
                        affected: vec![endpoint],
                        severity: (avg / 1000.0).min(1.0), // Normalize to 0-1
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Detect fault patterns
    fn detect_fault_patterns(&self, buckets: &[MetricsBucket]) -> Vec<ChaosPattern> {
        let mut endpoint_faults: HashMap<String, usize> = HashMap::new();
        let mut total_events_per_endpoint: HashMap<String, usize> = HashMap::new();

        for bucket in buckets {
            for (endpoint, count) in &bucket.affected_endpoints {
                *total_events_per_endpoint.entry(endpoint.clone()).or_insert(0) += count;
            }
            for (fault_type, count) in &bucket.faults_by_type {
                // Track faults by endpoint (simplified - assumes fault type contains endpoint info)
                *endpoint_faults.entry(fault_type.clone()).or_insert(0) += count;
            }
        }

        endpoint_faults
            .into_iter()
            .filter_map(|(endpoint, fault_count)| {
                let total = total_events_per_endpoint.get(&endpoint).copied().unwrap_or(1);
                let fault_rate = fault_count as f64 / total as f64;

                // High fault rate if > 20%
                if fault_rate > 0.2 {
                    Some(ChaosPattern {
                        pattern_type: "high_fault_rate".to_string(),
                        frequency: fault_rate,
                        affected: vec![endpoint],
                        severity: fault_rate.min(1.0),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Detect rate limit patterns
    fn detect_rate_limit_patterns(&self, buckets: &[MetricsBucket]) -> Vec<ChaosPattern> {
        let total_violations: usize = buckets.iter().map(|b| b.rate_limit_violations).sum();
        let total_events: usize = buckets.iter().map(|b| b.total_events).sum();

        if total_events == 0 {
            return Vec::new();
        }

        let violation_rate = total_violations as f64 / total_events as f64;

        if violation_rate > 0.1 {
            vec![ChaosPattern {
                pattern_type: "frequent_rate_limits".to_string(),
                frequency: violation_rate,
                affected: vec!["global".to_string()],
                severity: violation_rate.min(1.0),
            }]
        } else {
            Vec::new()
        }
    }

    /// Detect time-based patterns
    fn detect_time_patterns(&self, buckets: &[MetricsBucket]) -> Vec<ChaosPattern> {
        // Look for patterns like "more errors during certain hours"
        // This is a simplified implementation
        if buckets.len() < 10 {
            return Vec::new();
        }

        let mut patterns = Vec::new();

        // Check if there's an increasing trend in faults
        let first_half = &buckets[..buckets.len() / 2];
        let second_half = &buckets[buckets.len() / 2..];

        let first_avg_faults: f64 = first_half.iter().map(|b| b.total_faults).sum::<usize>() as f64
            / first_half.len() as f64;
        let second_avg_faults: f64 = second_half.iter().map(|b| b.total_faults).sum::<usize>()
            as f64
            / second_half.len() as f64;

        if second_avg_faults > first_avg_faults * 1.5 {
            patterns.push(ChaosPattern {
                pattern_type: "increasing_fault_trend".to_string(),
                frequency: 1.0,
                affected: vec!["system".to_string()],
                severity: ((second_avg_faults - first_avg_faults) / first_avg_faults.max(1.0))
                    .min(1.0),
            });
        }

        patterns
    }

    /// Detect system weaknesses
    fn detect_weaknesses(
        &self,
        buckets: &[MetricsBucket],
        impact: &ChaosImpact,
    ) -> Vec<SystemWeakness> {
        let mut weaknesses = Vec::new();

        // Weakness 1: No chaos testing on some endpoints
        if let Some(coverage_weakness) = self.detect_coverage_weakness(buckets) {
            weaknesses.push(coverage_weakness);
        }

        // Weakness 2: High impact from chaos (system not resilient)
        if impact.severity_score > 0.7 {
            weaknesses.push(SystemWeakness {
                weakness_type: "low_resilience".to_string(),
                endpoints: impact.top_affected_endpoints.iter().map(|(ep, _)| ep.clone()).collect(),
                severity: impact.severity_score,
                evidence: {
                    let mut map = HashMap::new();
                    map.insert("severity_score".to_string(), impact.severity_score);
                    map.insert("degradation_percent".to_string(), impact.avg_degradation_percent);
                    map
                },
            });
        }

        // Weakness 3: Insufficient fault coverage
        if self.detect_insufficient_fault_coverage(buckets) {
            weaknesses.push(SystemWeakness {
                weakness_type: "insufficient_fault_coverage".to_string(),
                endpoints: vec![],
                severity: 0.6,
                evidence: HashMap::new(),
            });
        }

        weaknesses
    }

    /// Detect coverage weakness
    fn detect_coverage_weakness(&self, buckets: &[MetricsBucket]) -> Option<SystemWeakness> {
        if buckets.is_empty() {
            return Some(SystemWeakness {
                weakness_type: "no_chaos_testing".to_string(),
                endpoints: vec![],
                severity: 0.8,
                evidence: HashMap::new(),
            });
        }

        None
    }

    /// Detect insufficient fault coverage
    fn detect_insufficient_fault_coverage(&self, buckets: &[MetricsBucket]) -> bool {
        let fault_types: std::collections::HashSet<_> =
            buckets.iter().flat_map(|b| b.faults_by_type.keys()).collect();

        // Expect at least 3 different fault types for good coverage
        fault_types.len() < 3
    }

    /// Generate recommendations from patterns
    fn recommendations_from_patterns(&self, patterns: &[ChaosPattern]) -> Vec<Recommendation> {
        patterns
            .iter()
            .filter_map(|pattern| self.pattern_to_recommendation(pattern))
            .collect()
    }

    /// Convert pattern to recommendation
    fn pattern_to_recommendation(&self, pattern: &ChaosPattern) -> Option<Recommendation> {
        match pattern.pattern_type.as_str() {
            "high_latency" => Some(self.create_latency_recommendation(pattern)),
            "high_fault_rate" => Some(self.create_fault_recommendation(pattern)),
            "frequent_rate_limits" => Some(self.create_rate_limit_recommendation(pattern)),
            "increasing_fault_trend" => Some(self.create_trend_recommendation(pattern)),
            _ => None,
        }
    }

    /// Create latency recommendation
    fn create_latency_recommendation(&self, pattern: &ChaosPattern) -> Recommendation {
        let endpoint = pattern.affected.first().map(|s| s.as_str()).unwrap_or("unknown");

        Recommendation {
            id: format!("rec-latency-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::Latency,
            severity: if pattern.severity > 0.7 {
                RecommendationSeverity::High
            } else {
                RecommendationSeverity::Medium
            },
            confidence: Confidence::new(0.85),
            title: format!("Increase latency testing for endpoint: {}", endpoint),
            description: format!(
                "Endpoint {} shows high average latency ({:.0}ms) under chaos conditions",
                endpoint,
                pattern.severity * 1000.0
            ),
            rationale: "High latency detected consistently across chaos experiments. \
                        This indicates the endpoint may be sensitive to delays and needs \
                        more comprehensive latency testing."
                .to_string(),
            action: format!(
                "Add more aggressive latency scenarios for endpoint {}. \
                 Test with latencies up to {}ms to validate timeout handling.",
                endpoint,
                (pattern.severity * 2000.0) as u64
            ),
            example: Some(format!(
                "mockforge serve --chaos --chaos-latency-ms {} --chaos-latency-probability 0.8",
                (pattern.severity * 1500.0) as u64
            )),
            affected_endpoints: pattern.affected.clone(),
            metrics: {
                let mut map = HashMap::new();
                map.insert("avg_latency_ms".to_string(), pattern.severity * 1000.0);
                map.insert("frequency".to_string(), pattern.frequency);
                map
            },
            generated_at: Utc::now(),
            expected_impact: pattern.severity * 0.8,
        }
    }

    /// Create fault recommendation
    fn create_fault_recommendation(&self, pattern: &ChaosPattern) -> Recommendation {
        let endpoint = pattern.affected.first().map(|s| s.as_str()).unwrap_or("unknown");

        Recommendation {
            id: format!("rec-fault-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::FaultInjection,
            severity: if pattern.severity > 0.5 {
                RecommendationSeverity::High
            } else {
                RecommendationSeverity::Medium
            },
            confidence: Confidence::new(0.80),
            title: format!("Endpoint {} shows high fault sensitivity", endpoint),
            description: format!(
                "Fault rate of {:.1}% detected for endpoint {}",
                pattern.frequency * 100.0,
                endpoint
            ),
            rationale: "High fault rate indicates insufficient error handling or retry logic. \
                        Testing with more diverse fault types is recommended."
                .to_string(),
            action: format!(
                "Implement comprehensive error handling for endpoint {}. \
                 Test with multiple fault types (500, 502, 503, 504, connection errors).",
                endpoint
            ),
            example: Some(
                "mockforge serve --chaos --chaos-http-errors '500,502,503,504' \
                 --chaos-http-error-probability 0.3"
                    .to_string(),
            ),
            affected_endpoints: pattern.affected.clone(),
            metrics: {
                let mut map = HashMap::new();
                map.insert("fault_rate".to_string(), pattern.frequency);
                map.insert("severity".to_string(), pattern.severity);
                map
            },
            generated_at: Utc::now(),
            expected_impact: pattern.severity,
        }
    }

    /// Create rate limit recommendation
    fn create_rate_limit_recommendation(&self, pattern: &ChaosPattern) -> Recommendation {
        Recommendation {
            id: format!("rec-ratelimit-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::RateLimit,
            severity: RecommendationSeverity::Medium,
            confidence: Confidence::new(0.75),
            title: "Frequent rate limit violations detected".to_string(),
            description: format!(
                "Rate limit violations occurring at {:.1}% of requests",
                pattern.frequency * 100.0
            ),
            rationale: "High rate of rate limiting indicates need for better backpressure \
                        handling and retry logic with exponential backoff."
                .to_string(),
            action: "Implement proper retry logic with exponential backoff. \
                     Test with more aggressive rate limits to validate behavior."
                .to_string(),
            example: Some(
                "mockforge serve --chaos --chaos-rate-limit 10 --chaos-scenario peak_traffic"
                    .to_string(),
            ),
            affected_endpoints: pattern.affected.clone(),
            metrics: {
                let mut map = HashMap::new();
                map.insert("violation_rate".to_string(), pattern.frequency);
                map
            },
            generated_at: Utc::now(),
            expected_impact: 0.6,
        }
    }

    /// Create trend recommendation
    fn create_trend_recommendation(&self, pattern: &ChaosPattern) -> Recommendation {
        Recommendation {
            id: format!("rec-trend-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::Scenario,
            severity: RecommendationSeverity::High,
            confidence: Confidence::new(0.70),
            title: "Increasing fault trend detected - system degradation".to_string(),
            description: "Fault rate increasing over time, indicating system degradation \
                          or cascading failures."
                .to_string(),
            rationale: "Increasing fault trends suggest lack of circuit breaker or bulkhead \
                        patterns. System may be experiencing cascading failures."
                .to_string(),
            action: "Implement circuit breaker and bulkhead patterns. \
                     Test with cascading failure scenarios."
                .to_string(),
            example: Some("mockforge serve --chaos --chaos-scenario cascading_failure".to_string()),
            affected_endpoints: pattern.affected.clone(),
            metrics: {
                let mut map = HashMap::new();
                map.insert("severity".to_string(), pattern.severity);
                map
            },
            generated_at: Utc::now(),
            expected_impact: 0.9,
        }
    }

    /// Generate recommendations from weaknesses
    fn recommendations_from_weaknesses(
        &self,
        weaknesses: &[SystemWeakness],
    ) -> Vec<Recommendation> {
        weaknesses
            .iter()
            .filter_map(|weakness| self.weakness_to_recommendation(weakness))
            .collect()
    }

    /// Convert weakness to recommendation
    fn weakness_to_recommendation(&self, weakness: &SystemWeakness) -> Option<Recommendation> {
        match weakness.weakness_type.as_str() {
            "no_chaos_testing" => Some(self.create_no_testing_recommendation()),
            "low_resilience" => Some(self.create_resilience_recommendation(weakness)),
            "insufficient_fault_coverage" => Some(self.create_coverage_recommendation()),
            _ => None,
        }
    }

    /// Create no testing recommendation
    fn create_no_testing_recommendation(&self) -> Recommendation {
        Recommendation {
            id: format!("rec-start-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::Coverage,
            severity: RecommendationSeverity::Critical,
            confidence: Confidence::new(1.0),
            title: "Start chaos engineering testing".to_string(),
            description: "No chaos testing detected. Begin with basic scenarios to build \
                          confidence in system resilience."
                .to_string(),
            rationale: "Without chaos testing, you cannot validate how your system behaves \
                        under failure conditions."
                .to_string(),
            action: "Start with the 'network_degradation' scenario to test basic resilience."
                .to_string(),
            example: Some(
                "mockforge serve --chaos --chaos-scenario network_degradation".to_string(),
            ),
            affected_endpoints: vec![],
            metrics: HashMap::new(),
            generated_at: Utc::now(),
            expected_impact: 1.0,
        }
    }

    /// Create resilience recommendation
    fn create_resilience_recommendation(&self, weakness: &SystemWeakness) -> Recommendation {
        Recommendation {
            id: format!("rec-resilience-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::CircuitBreaker,
            severity: RecommendationSeverity::Critical,
            confidence: Confidence::new(0.85),
            title: "System shows low resilience - implement resilience patterns".to_string(),
            description: format!(
                "System degradation of {:.1}% under chaos - resilience patterns needed",
                weakness.evidence.get("degradation_percent").unwrap_or(&0.0)
            ),
            rationale: "High system degradation indicates missing resilience patterns like \
                        circuit breakers, bulkheads, and retry logic."
                .to_string(),
            action: "Implement circuit breaker and bulkhead patterns for critical endpoints. \
                     Add retry logic with exponential backoff."
                .to_string(),
            example: Some(
                "# Test with circuit breaker scenario\n\
                 mockforge serve --chaos --chaos-scenario cascading_failure"
                    .to_string(),
            ),
            affected_endpoints: weakness.endpoints.clone(),
            metrics: weakness.evidence.clone(),
            generated_at: Utc::now(),
            expected_impact: 0.95,
        }
    }

    /// Create coverage recommendation
    fn create_coverage_recommendation(&self) -> Recommendation {
        Recommendation {
            id: format!("rec-coverage-{}", uuid::Uuid::new_v4()),
            category: RecommendationCategory::Coverage,
            severity: RecommendationSeverity::High,
            confidence: Confidence::new(0.80),
            title: "Insufficient fault type coverage".to_string(),
            description: "Testing with limited fault types. Expand coverage to include \
                          multiple error conditions."
                .to_string(),
            rationale: "Comprehensive chaos testing should include various fault types: \
                        HTTP errors (500, 502, 503, 504), connection errors, and timeouts."
                .to_string(),
            action: "Add diverse fault injection scenarios covering all major failure modes."
                .to_string(),
            example: Some(
                "mockforge serve --chaos --chaos-scenario service_instability".to_string(),
            ),
            affected_endpoints: vec![],
            metrics: HashMap::new(),
            generated_at: Utc::now(),
            expected_impact: 0.7,
        }
    }

    /// Generate coverage recommendations
    fn coverage_recommendations(
        &self,
        buckets: &[MetricsBucket],
        _impact: &ChaosImpact,
    ) -> Vec<Recommendation> {
        let mut recs = Vec::new();

        // Check protocol coverage
        let protocols_tested: std::collections::HashSet<_> =
            buckets.iter().flat_map(|b| b.protocol_events.keys()).collect();

        if protocols_tested.is_empty() || protocols_tested.len() < 2 {
            recs.push(Recommendation {
                id: format!("rec-protocol-{}", uuid::Uuid::new_v4()),
                category: RecommendationCategory::Coverage,
                severity: RecommendationSeverity::Medium,
                confidence: Confidence::new(0.75),
                title: "Expand protocol-specific chaos testing".to_string(),
                description: "Limited protocol coverage. Test chaos scenarios across \
                              HTTP, gRPC, WebSocket, and GraphQL."
                    .to_string(),
                rationale: "Different protocols have different failure modes. \
                            Comprehensive testing should cover all protocols in use."
                    .to_string(),
                action: "Enable protocol-specific chaos scenarios.".to_string(),
                example: Some(
                    "# Test gRPC chaos\n\
                     mockforge serve --chaos --grpc-port 50051"
                        .to_string(),
                ),
                affected_endpoints: vec![],
                metrics: HashMap::new(),
                generated_at: Utc::now(),
                expected_impact: 0.6,
            });
        }

        recs
    }

    /// Generate scenario recommendations
    fn scenario_recommendations(&self, impact: &ChaosImpact) -> Vec<Recommendation> {
        let mut recs = Vec::new();

        // Recommend progressive chaos testing
        if impact.total_events < 100 {
            recs.push(Recommendation {
                id: format!("rec-progressive-{}", uuid::Uuid::new_v4()),
                category: RecommendationCategory::Scenario,
                severity: RecommendationSeverity::Medium,
                confidence: Confidence::new(0.70),
                title: "Implement progressive chaos testing".to_string(),
                description: "Start with light chaos and gradually increase intensity \
                              to identify breaking points."
                    .to_string(),
                rationale: "Progressive testing helps identify at what point your system \
                            starts to degrade, allowing you to set appropriate limits."
                    .to_string(),
                action: "Run chaos scenarios in order of increasing intensity: \
                         network_degradation → service_instability → cascading_failure"
                    .to_string(),
                example: Some(
                    "# Phase 1: Light chaos\n\
                     mockforge serve --chaos --chaos-scenario network_degradation\n\n\
                     # Phase 2: Medium chaos\n\
                     mockforge serve --chaos --chaos-scenario service_instability\n\n\
                     # Phase 3: Heavy chaos\n\
                     mockforge serve --chaos --chaos-scenario cascading_failure"
                        .to_string(),
                ),
                affected_endpoints: vec![],
                metrics: HashMap::new(),
                generated_at: Utc::now(),
                expected_impact: 0.75,
            });
        }

        recs
    }

    /// Get all current recommendations
    pub fn get_recommendations(&self) -> Vec<Recommendation> {
        self.recommendations.read().clone()
    }

    /// Get recommendations by category
    pub fn get_recommendations_by_category(
        &self,
        category: RecommendationCategory,
    ) -> Vec<Recommendation> {
        self.recommendations
            .read()
            .iter()
            .filter(|r| r.category == category)
            .cloned()
            .collect()
    }

    /// Get recommendations by severity
    pub fn get_recommendations_by_severity(
        &self,
        min_severity: RecommendationSeverity,
    ) -> Vec<Recommendation> {
        self.recommendations
            .read()
            .iter()
            .filter(|r| r.severity >= min_severity)
            .cloned()
            .collect()
    }

    /// Clear all recommendations
    pub fn clear(&self) {
        self.recommendations.write().clear();
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    #[test]
    fn test_confidence_creation() {
        let conf = Confidence::new(0.8);
        assert_eq!(conf.value(), 0.8);
        assert!(conf.is_high());
        assert!(!conf.is_medium());
        assert!(!conf.is_low());
    }

    #[test]
    fn test_confidence_clamping() {
        let conf = Confidence::new(1.5);
        assert_eq!(conf.value(), 1.0);

        let conf = Confidence::new(-0.5);
        assert_eq!(conf.value(), 0.0);
    }

    #[test]
    fn test_recommendation_score() {
        let rec = Recommendation {
            id: "test".to_string(),
            category: RecommendationCategory::Latency,
            severity: RecommendationSeverity::High,
            confidence: Confidence::new(0.9),
            title: "Test".to_string(),
            description: "Test".to_string(),
            rationale: "Test".to_string(),
            action: "Test".to_string(),
            example: None,
            affected_endpoints: vec![],
            metrics: HashMap::new(),
            generated_at: Utc::now(),
            expected_impact: 0.8,
        };

        let score = rec.score();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_engine_creation() {
        let engine = RecommendationEngine::new();
        let recs = engine.get_recommendations();
        assert_eq!(recs.len(), 0);
    }

    #[test]
    fn test_detect_latency_patterns() {
        let engine = RecommendationEngine::new();

        let mut bucket = MetricsBucket::new(Utc::now(), crate::analytics::TimeBucket::Minute);
        bucket.avg_latency_ms = 800.0;
        bucket.affected_endpoints.insert("/api/slow".to_string(), 10);

        let patterns = engine.detect_latency_patterns(&[bucket]);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, "high_latency");
    }

    #[test]
    fn test_no_chaos_recommendation() {
        let engine = RecommendationEngine::new();
        let impact = ChaosImpact::from_buckets(&[]);

        let recs = engine.analyze_and_recommend(&[], &impact);

        // Should recommend starting chaos testing
        assert!(!recs.is_empty());
        assert!(recs.iter().any(|r| r.category == RecommendationCategory::Coverage));
    }
}

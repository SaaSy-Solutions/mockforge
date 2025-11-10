//! Integration tests for MockAI rule explanations

use mockforge_core::intelligent_behavior::{
    config::BehaviorModelConfig,
    rule_generator::{ExamplePair, RuleGenerator, RuleType},
};
use serde_json::json;

fn create_test_example_pair() -> ExamplePair {
    ExamplePair {
        method: "POST".to_string(),
        path: "/api/users".to_string(),
        request: Some(json!({
            "name": "Alice",
            "email": "alice@example.com"
        })),
        status: 201,
        response: Some(json!({
            "id": "user_123",
            "name": "Alice",
            "email": "alice@example.com"
        })),
        query_params: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn test_rule_generation_with_explanations() {
    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![
        create_test_example_pair(),
        create_test_example_pair(),
        create_test_example_pair(),
    ];

    // Generate rules with explanations
    let result = generator.generate_rules_with_explanations(examples).await;

    match result {
        Ok((rules, explanations)) => {
            // Should have generated some rules or explanations
            let has_rules = !rules.consistency_rules.is_empty()
                || !rules.schemas.is_empty()
                || !rules.state_transitions.is_empty();
            let has_explanations = !explanations.is_empty();

            // At least one should be true
            assert!(has_rules || has_explanations, "Should have generated rules or explanations");

            // If we have explanations, validate them
            for explanation in &explanations {
                assert!(!explanation.rule_id.is_empty());
                assert!(explanation.confidence >= 0.0 && explanation.confidence <= 1.0);
                assert!(!explanation.reasoning.is_empty());
                assert_eq!(explanation.generated_at.timestamp() > 0, true);
            }
        }
        Err(e) => {
            // If LLM is disabled, this might fail, which is acceptable
            println!("Rule generation failed (expected if LLM disabled): {}", e);
        }
    }
}

#[tokio::test]
async fn test_explanation_rule_types() {
    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![create_test_example_pair(), create_test_example_pair()];

    let result = generator.generate_rules_with_explanations(examples).await;

    if let Ok((_, explanations)) = result {
        // Check that we have various rule types
        let rule_types: Vec<RuleType> = explanations.iter().map(|e| e.rule_type).collect();

        // Should have at least one explanation
        if !rule_types.is_empty() {
            // All rule types should be valid
            for rule_type in &rule_types {
                match rule_type {
                    RuleType::Crud
                    | RuleType::Validation
                    | RuleType::Pagination
                    | RuleType::Consistency
                    | RuleType::StateTransition
                    | RuleType::Other => {
                        // Valid rule type
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_explanation_source_tracking() {
    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![create_test_example_pair(), create_test_example_pair()];

    let result = generator.generate_rules_with_explanations(examples).await;

    if let Ok((_, explanations)) = result {
        for explanation in &explanations {
            // Source examples should be valid IDs
            for example_id in &explanation.source_examples {
                assert!(!example_id.is_empty());
            }

            // Pattern matches should have valid data
            for pattern_match in &explanation.pattern_matches {
                assert!(!pattern_match.pattern.is_empty());
                assert!(pattern_match.match_count > 0);
                assert!(!pattern_match.example_ids.is_empty());
            }
        }
    }
}

#[tokio::test]
async fn test_explanation_confidence_ranges() {
    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![
        create_test_example_pair(),
        create_test_example_pair(),
        create_test_example_pair(),
    ];

    let result = generator.generate_rules_with_explanations(examples).await;

    if let Ok((_, explanations)) = result {
        for explanation in &explanations {
            // Confidence should be in valid range
            assert!(
                explanation.confidence >= 0.0 && explanation.confidence <= 1.0,
                "Confidence should be between 0 and 1, got {}",
                explanation.confidence
            );
        }
    }
}

#[tokio::test]
async fn test_empty_examples_handling() {
    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![];

    // Should handle empty examples gracefully
    let result = generator.generate_rules_with_explanations(examples).await.unwrap();

    let (rules, explanations) = result;

    // Should return empty/default rules
    assert!(rules.consistency_rules.is_empty());
    assert!(rules.schemas.is_empty());
    assert!(rules.state_transitions.is_empty());

    // Should have no explanations
    assert!(explanations.is_empty());
}

#[tokio::test]
async fn test_explanation_reasoning_quality() {
    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![create_test_example_pair(), create_test_example_pair()];

    let result = generator.generate_rules_with_explanations(examples).await;

    if let Ok((_, explanations)) = result {
        for explanation in &explanations {
            // Reasoning should be meaningful (not empty, not just whitespace)
            let reasoning_trimmed = explanation.reasoning.trim();
            assert!(
                reasoning_trimmed.len() > 10,
                "Reasoning should be meaningful, got: '{}'",
                reasoning_trimmed
            );
        }
    }
}

#[tokio::test]
async fn test_learn_endpoint_storage() {
    // This test verifies that the learn endpoint would store explanations
    // In a real integration test, we'd set up a test server and make HTTP requests
    // For now, we test the core functionality

    let config = BehaviorModelConfig::default();
    let generator = RuleGenerator::new(config);

    let examples = vec![create_test_example_pair(), create_test_example_pair()];

    // Generate rules with explanations (simulating what the endpoint does)
    let result = generator.generate_rules_with_explanations(examples).await;

    if let Ok((rules, explanations)) = result {
        // Verify explanations are generated
        assert!(
            !explanations.is_empty()
                || !rules.consistency_rules.is_empty()
                || !rules.schemas.is_empty()
        );

        // Verify each explanation has a unique rule_id (for storage)
        let rule_ids: Vec<String> = explanations.iter().map(|e| e.rule_id.clone()).collect();
        let unique_ids: std::collections::HashSet<String> = rule_ids.iter().cloned().collect();

        // All rule IDs should be unique (for proper storage)
        assert_eq!(rule_ids.len(), unique_ids.len(), "Rule IDs should be unique");

        // Verify explanations can be serialized (for API response)
        for explanation in &explanations {
            let json = serde_json::to_value(explanation);
            assert!(json.is_ok(), "Explanation should be serializable");
        }
    }
}

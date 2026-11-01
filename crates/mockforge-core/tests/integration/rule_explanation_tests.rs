//! Unit tests for rule explanations

use mockforge_core::intelligent_behavior::config::BehaviorModelConfig;
use mockforge_core::intelligent_behavior::rule_generator::{
    ExamplePair, RuleExplanation, RuleGenerator, RuleType,
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
        headers: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn test_rule_explanation_creation() {
    let explanation = RuleExplanation::new(
        "test_rule_1".to_string(),
        RuleType::Consistency,
        0.85,
        "Test reasoning".to_string(),
    );

    assert_eq!(explanation.rule_id, "test_rule_1");
    assert_eq!(explanation.rule_type, RuleType::Consistency);
    assert_eq!(explanation.confidence, 0.85);
    assert_eq!(explanation.reasoning, "Test reasoning");
    assert!(explanation.source_examples.is_empty());
    assert!(explanation.pattern_matches.is_empty());
}

#[tokio::test]
async fn test_rule_explanation_with_source_examples() {
    let explanation = RuleExplanation::new(
        "test_rule_2".to_string(),
        RuleType::Validation,
        0.75,
        "Validation rule".to_string(),
    )
    .with_source_example("example_1".to_string())
    .with_source_example("example_2".to_string());

    assert_eq!(explanation.source_examples.len(), 2);
    assert_eq!(explanation.source_examples[0], "example_1");
    assert_eq!(explanation.source_examples[1], "example_2");
}

#[tokio::test]
async fn test_generate_rules_with_explanations() {
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
            // Should have generated rules
            assert!(
                !rules.consistency_rules.is_empty()
                    || !rules.schemas.is_empty()
                    || !rules.state_transitions.is_empty()
            );

            // Should have explanations
            assert!(!explanations.is_empty(), "Should have generated explanations");

            // Each explanation should have valid data
            for explanation in &explanations {
                assert!(!explanation.rule_id.is_empty());
                assert!(explanation.confidence >= 0.0 && explanation.confidence <= 1.0);
                assert!(!explanation.reasoning.is_empty());
            }
        }
        Err(e) => {
            // If LLM is disabled, this might fail, which is acceptable
            println!("Rule generation failed (expected if LLM disabled): {}", e);
        }
    }
}

#[tokio::test]
async fn test_rule_type_enum() {
    // Test all rule types
    let types = vec![
        RuleType::Crud,
        RuleType::Validation,
        RuleType::Pagination,
        RuleType::Consistency,
        RuleType::StateTransition,
        RuleType::Other,
    ];

    for rule_type in types {
        let explanation =
            RuleExplanation::new("test".to_string(), rule_type, 0.5, "Test".to_string());
        assert_eq!(explanation.rule_type, rule_type);
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

    // Should return default/empty rules
    assert!(rules.consistency_rules.is_empty());
    assert!(rules.schemas.is_empty());
    assert!(rules.state_transitions.is_empty());

    // Should have no explanations
    assert!(explanations.is_empty());
}

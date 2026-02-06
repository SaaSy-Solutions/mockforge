//! Verify path parameter resolution for $ref parameters
#[cfg(test)]
mod tests {
    use mockforge_bench::request_gen::RequestGenerator;
    use mockforge_bench::spec_parser::SpecParser;
    use std::path::PathBuf;

    #[tokio::test]
    async fn verify_ref_path_params_are_resolved() {
        let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("billing_subscriptions_v1.json");

        let parser = SpecParser::from_file(&spec_path).await.unwrap();
        let operations = parser.get_operations();

        // Find an operation with {id} in its path
        let op_with_id = operations
            .iter()
            .find(|op| op.path.contains("{id}"))
            .expect("Should have an operation with {id} in path");

        eprintln!("Operation: {} {}", op_with_id.method, op_with_id.path);
        eprintln!("Parameters count: {}", op_with_id.operation.parameters.len());

        // Generate template
        let template =
            RequestGenerator::generate_template(op_with_id).expect("Should generate template");

        eprintln!("Path params: {:?}", template.path_params);

        let generated_path = template.generate_path();
        eprintln!("Generated path: {}", generated_path);

        // The critical assertion: {id} should be replaced with a concrete value
        assert!(
            !generated_path.contains("{id}"),
            "Path should NOT contain literal {{id}} - should be replaced with a value like '1'. Got: {}",
            generated_path
        );

        assert!(
            generated_path.contains("/1") || generated_path.contains("/test-value"),
            "Path should contain the resolved parameter value. Got: {}",
            generated_path
        );
    }
}

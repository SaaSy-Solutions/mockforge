//! Temporary test to dump generated k6 script for manual inspection
//! Run with: cargo test -p mockforge-bench dump_generated_script -- --nocapture

#[cfg(test)]
mod tests {
    use mockforge_bench::k6_gen::{K6Config, K6ScriptGenerator};
    use mockforge_bench::request_gen::RequestGenerator;
    use mockforge_bench::scenarios::LoadScenario;
    use mockforge_bench::security_payloads::{
        SecurityPayloads, SecurityTestConfig, SecurityTestGenerator,
    };
    use mockforge_bench::spec_parser::SpecParser;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn dump_generated_script() {
        let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("billing_subscriptions_v1.json");

        let parser = SpecParser::from_file(&spec_path).await.unwrap();
        let operations = parser.get_operations();
        let templates: Vec<_> = operations
            .iter()
            .map(RequestGenerator::generate_template)
            .collect::<mockforge_bench::error::Result<Vec<_>>>()
            .unwrap();

        let config = K6Config {
            target_url: "https://api-m.sandbox.paypal.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 10,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: Some("Bearer test-token".to_string()),
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
            security_testing_enabled: true,
        };

        let generator = K6ScriptGenerator::new(config, templates);
        let mut script = generator.generate().unwrap();

        // Simulate generate_enhanced_script
        let sec_config = SecurityTestConfig::default().enable();
        let payloads = SecurityPayloads::get_payloads(&sec_config);
        let mut code = String::new();
        code.push_str(&SecurityTestGenerator::generate_payload_selection(&payloads, false));
        code.push('\n');
        code.push_str(&SecurityTestGenerator::generate_apply_payload(&[]));
        code.push('\n');

        if let Some(pos) = script.find("export const options") {
            script.insert_str(pos, &format!("\n// === Security Testing ===\n{}\n", code));
        }

        // Print just the default function section
        if let Some(start) = script.find("export default function") {
            let section = &script[start..];
            if let Some(end) = section.find("\nexport function handleSummary") {
                eprintln!("\n=== GENERATED export default function() ===\n");
                eprintln!("{}", &section[..end]);
                eprintln!("\n=== END ===\n");
            }
        }
    }
}

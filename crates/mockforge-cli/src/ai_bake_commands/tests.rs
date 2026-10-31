use super::{
    args::{BakeArgs, Provider},
    document, storage,
};
use serde_json::{json, Value};
use std::path::PathBuf;

fn args(output: PathBuf) -> BakeArgs {
    BakeArgs {
        prompt_file: "prompt.txt".into(),
        output,
        provider: Provider::Ollama,
        model: "fixture-model".into(),
        endpoint: None,
        api_key_env: None,
        seed: Some(42),
        temperature: 0.0,
        max_tokens: 100,
        timeout: 5,
        method: "GET".into(),
        path: "/fixture".into(),
        status: 200,
    }
}

#[test]
fn all_json_values_survive_yaml_and_json_as_static_serve_routes() {
    let directory = tempfile::tempdir().unwrap();
    for extension in ["json", "yaml"] {
        for (index, body) in [
            json!(null),
            json!(false),
            json!(42),
            json!("{{uuid}}"),
            json!([1, 2]),
            json!({"id":1}),
        ]
        .into_iter()
        .enumerate()
        {
            let args = args(directory.path().join(format!("{index}.{extension}")));
            let identity = document::identity(&args, "fixture prompt");
            let baked = document::build(&args, body.clone(), &identity).unwrap();
            storage::publish(&args.output, &baked, &identity).unwrap();
            let loaded = storage::load(&args.output, &identity).unwrap().unwrap();
            let config: mockforge_core::config::ServerConfig =
                serde_json::from_value(loaded).unwrap();
            assert_eq!(config.routes[0].response.body, Some(body));
            assert!(!config.http.response_template_expand);
            assert!(!config.mockai.enabled);
            assert!(!config.reality.enabled);
        }
    }
}

#[test]
fn publication_never_overwrites_a_winner_and_detects_modified_data() {
    let directory = tempfile::tempdir().unwrap();
    let args = args(directory.path().join("fixture.json"));
    let identity = document::identity(&args, "fixture prompt");
    let first = document::build(&args, json!({"winner":1}), &identity).unwrap();
    storage::publish(&args.output, &first, &identity).unwrap();
    let bytes = std::fs::read(&args.output).unwrap();
    let second = document::build(&args, json!({"winner":2}), &identity).unwrap();
    storage::publish(&args.output, &second, &identity).unwrap();
    assert_eq!(bytes, std::fs::read(&args.output).unwrap());
    assert!(storage::load(&args.output, "different request").is_err());
    let mut modified: Value = serde_json::from_slice(&bytes).unwrap();
    modified["routes"][0]["response"]["body"] = json!(99);
    std::fs::write(&args.output, serde_json::to_vec(&modified).unwrap()).unwrap();
    assert!(storage::load(&args.output, &identity).is_err());
    assert!(storage::publish(&args.output, &first, &identity).is_err());
}

#[test]
fn api_key_variable_name_and_output_location_do_not_change_identity() {
    let first = args("one.yaml".into());
    let mut second = args("two.json".into());
    second.api_key_env = Some("ROTATED_FIXTURE_KEY".into());
    assert_eq!(
        document::identity(&first, "same prompt"),
        document::identity(&second, "same prompt")
    );
    second.seed = Some(43);
    assert_ne!(
        document::identity(&first, "same prompt"),
        document::identity(&second, "same prompt")
    );
}

#[test]
fn cached_bakes_cannot_reenable_reality_presets() {
    let args = args(PathBuf::from("fixture.json"));
    let mut config = document::build(&args, serde_json::json!({"fixed":true}), "identity").unwrap();
    config.as_object_mut().unwrap().remove("reality");
    let manifest = config.as_object_mut().unwrap().remove("x-mockforge-ai-bake").unwrap();
    let digest = document::hash(&config);
    config["x-mockforge-ai-bake"] = manifest;
    config["x-mockforge-ai-bake"]["config_sha256"] = serde_json::json!(digest);
    assert!(document::verify(config, "identity").is_err());
}

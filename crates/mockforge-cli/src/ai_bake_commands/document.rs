//! Stable configuration plus a credential-free integrity manifest.
use super::{args::BakeArgs, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const MANIFEST: &str = "x-mockforge-ai-bake";

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    request_sha256: String,
    config_sha256: String,
}

pub(super) fn hash(value: &Value) -> String {
    let mut value = value.clone();
    value.sort_all_objects();
    hex::encode(
        ring::digest::digest(
            &ring::digest::SHA256,
            &serde_json::to_vec(&value).expect("JSON value serializes"),
        )
        .as_ref(),
    )
}

pub(super) fn identity(args: &BakeArgs, prompt: &str) -> String {
    hash(
        &json!({"version":1,"prompt":prompt,"provider":args.provider.name(),"model":args.model,
        "endpoint":args.endpoint,"seed":args.seed,"temperature":args.temperature,"max_tokens":args.max_tokens,
        "method":args.method.to_ascii_uppercase(),"path":args.path,"status":args.status}),
    )
}

pub(super) fn build(args: &BakeArgs, body: Value, identity: &str) -> Result<Value> {
    let mut config = json!({
        "http":{"enabled":true,"host":"127.0.0.1","port":3000,"response_template_expand":false,"cors":{"enabled":false}},
        "mockai":{"enabled":false},
        "routes":[{"method":args.method.to_ascii_uppercase(),"path":args.path,"response":{"status":args.status,
            "headers":{"content-type":"application/json"},"body":body}}]
    });
    for protocol in [
        "websocket",
        "grpc",
        "graphql",
        "mqtt",
        "smtp",
        "ftp",
        "amqp",
        "kafka",
        "tcp",
        "admin",
    ] {
        config[protocol] = json!({"enabled":false});
    }
    // Check the real serve configuration codec, including explicit JSON null.
    let _: mockforge_core::config::ServerConfig = serde_json::from_value(config.clone())?;
    let manifest = Manifest {
        version: 1,
        request_sha256: identity.into(),
        config_sha256: hash(&config),
    };
    config[MANIFEST] = serde_json::to_value(manifest)?;
    config.sort_all_objects();
    Ok(config)
}

pub(super) fn verify(mut config: Value, identity: &str) -> Result<Value> {
    let manifest = config
        .as_object_mut()
        .and_then(|object| object.remove(MANIFEST))
        .ok_or("Existing output has no AI bake integrity manifest; choose a new output path")?;
    let manifest: Manifest = serde_json::from_value(manifest)?;
    if manifest.version != 1 || manifest.request_sha256 != identity {
        return Err(
            "Existing fixture belongs to different generation inputs; choose a new output path"
                .into(),
        );
    }
    if manifest.config_sha256 != hash(&config) {
        return Err("Baked fixture integrity check failed; the file was not replaced".into());
    }
    let _: mockforge_core::config::ServerConfig = serde_json::from_value(config.clone())?;
    Ok(config)
}

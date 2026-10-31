//! Exercise the admin install handler with actual signed and unsigned WASM fixtures.
use axum::{
    body::{to_bytes, Body},
    http::Request,
    routing::post,
    Router,
};
use mockforge_plugin_core::{PluginAuthor, PluginId, PluginInfo, PluginManifest, PluginVersion};
use mockforge_plugin_loader::{generate_ed25519_keypair, sign_plugin_ed25519, TRUST_STORE_ENV};
use mockforge_ui::handlers::{install_plugin, AdminState};
use serde_json::{json, Value};
use std::{ffi::OsString, path::PathBuf};
use tower::ServiceExt;

const PLUGIN_ID: &str = "admin-signature-fixture";
// A real WASM module exporting one bounded memory (one page, maximum one page).
const WASM: &[u8] = b"\0asm\x01\0\0\0\x05\x04\x01\x01\x01\x01\x07\x0a\x01\x06memory\x02\0";

struct Fixture {
    _directory: tempfile::TempDir,
    plugin: PathBuf,
    store: PathBuf,
    public_key: String,
    private_key: Vec<u8>,
    previous_store: Option<OsString>,
    state: AdminState,
    app: Router,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let plugin = directory.path().join(PLUGIN_ID);
        std::fs::create_dir(&plugin).unwrap();
        let manifest = PluginManifest::new(PluginInfo::new(
            PluginId::new(PLUGIN_ID),
            PluginVersion::new(1, 0, 0),
            "Signature fixture",
            "Owned integration fixture",
            PluginAuthor::new("Fixture publisher"),
        ));
        std::fs::write(plugin.join("plugin.yaml"), serde_yaml::to_string(&manifest).unwrap())
            .unwrap();
        std::fs::write(plugin.join("plugin.wasm"), WASM).unwrap();
        let (private_key, key) = generate_ed25519_keypair().unwrap();
        let public_key = key.iter().map(|byte| format!("{byte:02x}")).collect();
        let store = directory.path().join("trusted-keys.json");
        let state = AdminState::new(
            None, None, None, None, true, 0, None, None, None, None, None, None, None, None,
        );
        let app = Router::new().route("/install", post(install_plugin)).with_state(state.clone());
        let previous_store = std::env::var_os(TRUST_STORE_ENV);
        let fixture = Self {
            _directory: directory,
            plugin,
            store,
            public_key,
            private_key,
            previous_store,
            state,
            app,
        };
        fixture.write_store(false, None);
        std::env::set_var(TRUST_STORE_ENV, &fixture.store);
        fixture
    }

    fn write_store(&self, revoked: bool, expiry: Option<&str>) {
        std::fs::write(
            &self.store,
            json!({"keys": [{
                "key_id": "fixture-publisher", "public_key": self.public_key,
                "revoked": revoked, "expires_at": expiry,
            }]})
            .to_string(),
        )
        .unwrap();
    }

    async fn install(&self, mut options: Value) -> Value {
        options["source"] = json!(self.plugin);
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/install")
                    .header("content-type", "application/json")
                    .body(Body::from(options.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn remove_installed(&self) {
        self.state
            .plugin_registry
            .write()
            .await
            .remove_plugin(&PluginId::new(PLUGIN_ID))
            .unwrap();
    }

    fn sign(&self) {
        sign_plugin_ed25519(&self.plugin, "fixture-publisher", &self.private_key).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        match self.previous_store.take() {
            Some(value) => std::env::set_var(TRUST_STORE_ENV, value),
            None => std::env::remove_var(TRUST_STORE_ENV),
        }
    }
}

fn rejected(value: &Value, message: &str) {
    assert_eq!(value["success"], false, "{value}");
    assert!(value["error"].as_str().unwrap().contains(message), "{value}");
}

// One sequential test owns the trust-store environment variable in this test process.
#[tokio::test]
async fn admin_install_enforces_publisher_trust_before_loading_or_replacing_plugins() {
    let fixture = Fixture::new();
    rejected(&fixture.install(json!({})).await, "No signature file");
    assert!(fixture.state.plugin_registry.read().await.list_plugins().is_empty());
    rejected(&fixture.install(json!({"skip_validation": true})).await, "Skipping validation");
    rejected(
        &fixture.install(json!({"no_verify": true})).await,
        "Disabling signature verification",
    );

    let unsigned = fixture.install(json!({"allow_unsigned": true})).await;
    assert_eq!(unsigned["success"], true, "{unsigned}");
    fixture.remove_installed().await;

    // A sibling with the same ID must never replace the verified source during loading.
    let shadow = fixture.plugin.parent().unwrap().join("unverified-shadow");
    std::fs::create_dir(&shadow).unwrap();
    let mut shadow_manifest: Value =
        serde_yaml::from_str(&std::fs::read_to_string(fixture.plugin.join("plugin.yaml")).unwrap())
            .unwrap();
    shadow_manifest["info"]["name"] = json!("Unverified sibling");
    std::fs::write(shadow.join("plugin.yaml"), serde_yaml::to_string(&shadow_manifest).unwrap())
        .unwrap();
    std::fs::write(shadow.join("plugin.wasm"), WASM).unwrap();
    fixture.sign();
    let signed = fixture.install(json!({})).await;
    assert_eq!(signed["success"], true, "{signed}");
    assert_eq!(
        fixture
            .state
            .plugin_registry
            .read()
            .await
            .get_plugin(&PluginId::new(PLUGIN_ID))
            .unwrap()
            .manifest
            .info
            .name,
        "Signature fixture"
    );
    std::fs::write(&fixture.store, json!({"keys": []}).to_string()).unwrap();
    rejected(&fixture.install(json!({"force": true})).await, "trusted keys");
    fixture.write_store(false, None);
    assert_eq!(fixture.state.plugin_registry.read().await.list_plugins().len(), 1);

    std::fs::write(fixture.plugin.join("plugin.wasm"), b"tampered").unwrap();
    rejected(
        &fixture.install(json!({"allow_unsigned": true, "force": true})).await,
        "modified",
    );
    assert_eq!(fixture.state.plugin_registry.read().await.list_plugins().len(), 1);
    std::fs::write(fixture.plugin.join("plugin.wasm"), WASM).unwrap();

    fixture.write_store(true, None);
    rejected(&fixture.install(json!({"force": true})).await, "revoked");
    fixture.write_store(false, Some("2000-01-01T00:00:00Z"));
    rejected(&fixture.install(json!({"force": true})).await, "expired");
    fixture.write_store(false, None);
    std::fs::write(fixture.plugin.join("plugin.sig"), "not a signature").unwrap();
    rejected(
        &fixture.install(json!({"allow_unsigned": true, "force": true})).await,
        "parse signature",
    );
    assert_eq!(fixture.state.plugin_registry.read().await.list_plugins().len(), 1);
}

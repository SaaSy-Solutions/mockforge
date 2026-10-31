//! Signature policy for the admin install path, before validation or execution.
use mockforge_plugin_loader::{
    resolve_trust_store_path, PluginLoaderConfig, SignatureVerifier, TrustStoreFile,
};
use std::path::Path;

pub(super) fn verified_plugin_config(
    plugin_root: &Path,
    allow_unsigned: bool,
) -> Result<PluginLoaderConfig, String> {
    let mut config = PluginLoaderConfig {
        allow_unsigned,
        ..Default::default()
    };
    if let Some(path) = resolve_trust_store_path(None) {
        TrustStoreFile::load(&path)
            .and_then(|store| store.apply_to(&mut config))
            .map_err(|error| format!("Failed to load plugin trust store: {error}"))?;
    }
    SignatureVerifier::new(&config)
        .verify_plugin_signature(plugin_root)
        .map_err(|error| format!("Plugin signature verification failed: {error}"))?;
    Ok(config)
}

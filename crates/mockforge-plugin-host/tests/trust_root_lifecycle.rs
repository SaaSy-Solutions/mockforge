//! End-to-end test for the trust-root refresh loop wired into the
//! plugin-host (issue #549). Acceptance criteria from the issue:
//!
//!   register → load (success) → revoke → reload (failure)
//!
//! This test stands up a real wiremock HTTP server as the registry,
//! a real `SignatureVerifier` in Required mode, and the real
//! refresh loop. The host actor is the production one; only the
//! upstream registry is faked.
//!
//! Two signing keys are generated up front:
//!   - `publisher-active` — present in the registry response throughout
//!   - `publisher-revoked` — present at first, then removed
//!
//! Both phases use the **same** WASM bytes (a minimal valid module)
//! so any difference in outcome can be attributed solely to the
//! trust-root state — not to the bytes or the manifest.
//!
//! Why current-thread runtime: the host actor owns a Wasmtime store
//! whose embedded `WasiCtx` is `!Send`, so the actor future itself
//! is `!Send` and cannot be spawned on a multi-thread runtime.
//! Matches the pattern in `host::tests` and `handlers::tests`.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use mockforge_plugin_host::{
    run_trust_root_refresh_loop, signing::build_signed_payload, Blocklist, Host, SignatureMode,
    SignatureVerifier, TrustRootCacheConfig, TrustStore,
};
use mockforge_plugin_loader::PluginLoaderConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Smallest valid WASM module — `\0asm` + version 1. Same fixture
/// the unit tests in `host::tests` use; chosen because it's the
/// minimum the loader accepts without WAT parsing.
const MINIMAL_WASM: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Deterministic-ish keypair from a 32-byte seed. Production code
/// uses `SigningKey::generate(&mut OsRng)` but tests want
/// reproducibility, so we seed manually. The seed isn't sensitive —
/// any signing key the test generates is thrown away when the
/// process exits.
fn fixture_keypair(seed: u8) -> (SigningKey, VerifyingKey) {
    let mut bytes = [0u8; 32];
    bytes[0] = seed;
    bytes[1] = 1;
    let sk = SigningKey::from_bytes(&bytes);
    let vk = sk.verifying_key();
    (sk, vk)
}

/// Build the JSON payload that `GET /api/v1/organizations/{org_id}
/// /trust-roots` returns. Matches `handlers::trust_roots::ListTrustRootsResponse`
/// shape (camelCase, with the per-row fields the cache reads).
fn trust_roots_response(entries: &[(&str, &VerifyingKey, bool)]) -> serde_json::Value {
    let trust_roots: Vec<serde_json::Value> = entries
        .iter()
        .map(|(id, vk, active)| {
            serde_json::json!({
                "id": id,
                "orgId": "00000000-0000-0000-0000-000000000000",
                "publicKeyB64": b64(vk.as_bytes()),
                "name": format!("test-key-{}", id),
                "active": active,
                "createdAt": "2026-01-01T00:00:00Z",
                "createdBy": null,
                "revokedAt": if *active { serde_json::Value::Null } else { serde_json::Value::String("2026-05-18T00:00:00Z".into()) },
                "revokedReason": null,
                "revokedBy": null,
            })
        })
        .collect();
    serde_json::json!({ "trustRoots": trust_roots })
}

#[test]
fn trust_root_lifecycle_register_load_revoke_reload() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    rt.block_on(async move {
        // ─── Fixtures ────────────────────────────────────────────
        let (sk_active, vk_active) = fixture_keypair(7);
        let (sk_revoked, vk_revoked) = fixture_keypair(13);
        let active_key_id = "publisher-active";
        let revoked_key_id = "publisher-revoked";

        // ─── Phase 1 fake registry: both keys are active. ────────
        let server = MockServer::start().await;
        let phase_one_body =
            trust_roots_response(&[(active_key_id, &vk_active, true), (revoked_key_id, &vk_revoked, true)]);
        let phase_one_mount = Mock::given(method("GET"))
            .and(path("/api/v1/organizations/x/trust-roots"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&phase_one_body))
            .mount_as_scoped(&server)
            .await;

        // ─── Host wiring ────────────────────────────────────────
        // Required mode so unsigned plugins are rejected — that's
        // what cloud production sets and it's what the issue is
        // about. The trust-root cache will populate the (initially
        // empty) store on its first tick.
        let trust_store = TrustStore::new();
        let verifier = SignatureVerifier::new(trust_store.clone(), SignatureMode::Required);
        let (host, actor, _bus) = Host::new(
            PluginLoaderConfig {
                allow_unsigned: true,
                skip_wasm_validation: true,
                ..Default::default()
            },
            verifier,
            Blocklist::new(),
            mockforge_plugin_host::PlatformTrustStore::new(),
        );

        // ─── Spawn the refresh loop. Short interval so the test
        // doesn't sit on the wall clock — the immediate-first-tick
        // semantics already cover the boot case, but the second
        // phase needs at least one extra tick to pick up the new
        // mock body.
        let cfg = TrustRootCacheConfig {
            url: format!("{}/api/v1/organizations/x/trust-roots", server.uri()),
            interval: std::time::Duration::from_millis(50),
            bearer_token: None,
        };
        let store_for_refresh = trust_store.clone();
        let refresh_handle = tokio::spawn(async move {
            run_trust_root_refresh_loop(cfg, store_for_refresh, |_| {}).await
        });

        // Drive the actor concurrently with the test body. The
        // actor future is !Send so it can't be `spawn`'d; instead
        // we drive it via select! and let the test body's
        // completion end the runtime.
        let outcome = tokio::select! {
            result = run_lifecycle_test(host, trust_store, sk_active, sk_revoked, active_key_id, revoked_key_id, &server, phase_one_mount, vk_active, vk_revoked) => result,
            _ = actor => panic!("actor exited before test body finished"),
        };
        refresh_handle.abort();
        outcome.expect("trust-root lifecycle test failed");
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_lifecycle_test(
    host: Host,
    trust_store: TrustStore,
    sk_active: SigningKey,
    sk_revoked: SigningKey,
    active_key_id: &str,
    revoked_key_id: &str,
    server: &MockServer,
    phase_one_mount: wiremock::MockGuard,
    vk_active: VerifyingKey,
    vk_revoked: VerifyingKey,
) -> Result<(), String> {
    // ─── Wait for phase-one refresh to populate the store. ───────
    // Without this the first load races the refresh loop and may
    // see an empty store (→ unknown_publisher_key) for reasons
    // unrelated to the test scenario.
    wait_for(
        || trust_store.get(active_key_id).is_some() && trust_store.get(revoked_key_id).is_some(),
        "phase-one trust-root population",
    )
    .await?;

    // ─── Phase 1: load with the "revoked" key (still active!) ───
    // Should succeed because both keys are in the active set.
    let revoked_sig = b64(&sk_revoked.sign(&build_signed_payload(MINIMAL_WASM, None)).to_bytes());
    host.load_plugin(
        "tenant-plugin",
        "1.0.0",
        serde_json::json!({}),
        MINIMAL_WASM.to_vec(),
        Some(revoked_sig.clone()),
        Some(revoked_key_id.to_string()),
        None,
    )
    .await
    .map_err(|err| format!("phase-1 load (signed by soon-to-be-revoked key) failed: {err}"))?;
    host.unload_plugin("tenant-plugin").await.expect("unload between phases");

    // Sanity check: a plugin signed by the always-active key also
    // loads in phase 1 — confirms the verifier path itself works
    // before we mutate the registry response.
    let active_sig = b64(&sk_active.sign(&build_signed_payload(MINIMAL_WASM, None)).to_bytes());
    host.load_plugin(
        "always-good",
        "1.0.0",
        serde_json::json!({}),
        MINIMAL_WASM.to_vec(),
        Some(active_sig.clone()),
        Some(active_key_id.to_string()),
        None,
    )
    .await
    .map_err(|err| format!("phase-1 load with active key failed: {err}"))?;

    // ─── Phase 2: registry revokes one key. ──────────────────────
    // Drop the phase-one mount (wiremock matches the first stage
    // first), then mount a response with only the active key.
    drop(phase_one_mount);
    let phase_two_body = trust_roots_response(&[
        (active_key_id, &vk_active, true),
        // Note `revoked = false` here: the API still surfaces the
        // row for audit history but `active: false` tells the
        // cache to drop it from the live store.
        (revoked_key_id, &vk_revoked, false),
    ]);
    let _phase_two_mount = Mock::given(method("GET"))
        .and(path("/api/v1/organizations/x/trust-roots"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&phase_two_body))
        .mount_as_scoped(server)
        .await;

    // Wait for the refresh loop to remove the revoked key from
    // the live store.
    wait_for(
        || trust_store.get(revoked_key_id).is_none() && trust_store.get(active_key_id).is_some(),
        "phase-two trust-root revocation propagation",
    )
    .await?;

    // ─── Phase 3: re-load with the revoked key — should fail. ───
    let err = host
        .load_plugin(
            "tenant-plugin",
            "1.0.0",
            serde_json::json!({}),
            MINIMAL_WASM.to_vec(),
            Some(revoked_sig),
            Some(revoked_key_id.to_string()),
            None,
        )
        .await
        .err()
        .ok_or("phase-3 load was expected to fail but succeeded")?;
    if err.code() != "unknown_publisher_key" {
        return Err(format!(
            "phase-3 load should fail with unknown_publisher_key after revocation; got {}",
            err.code()
        ));
    }

    // ─── Phase 4: confirm the always-active key still works. ────
    // This isolates the failure above — it's the *revocation* that
    // caused the rejection, not (for example) the cache dropping
    // every key on a refresh quirk.
    host.unload_plugin("always-good").await.expect("unload between phases 3 and 4");
    host.load_plugin(
        "always-good",
        "1.0.0",
        serde_json::json!({}),
        MINIMAL_WASM.to_vec(),
        Some(active_sig),
        Some(active_key_id.to_string()),
        None,
    )
    .await
    .map_err(|err| format!("phase-4 load with still-active key failed: {err}"))?;

    // ─── Phase 5: sweep flags the still-loaded `always-good`?
    // No — its key is still active. There's nothing for the
    // sweep to flag, so we expect an empty Vec. The "loaded
    // plugin signed by a revoked root → warn" path is covered
    // separately below in the dedicated test.
    let flagged = host.sweep_revoked_trust_roots().await.map_err(|err| err.to_string())?;
    if !flagged.is_empty() {
        return Err(format!(
            "phase-5 sweep should not have flagged anything (always-good key is active); \
             flagged {:?}",
            flagged
        ));
    }
    Ok(())
}

/// Verify that the periodic sweep flags a plugin whose
/// `publisher_key_id` is no longer in the live trust store, without
/// hot-unloading it (hot-unload is out of scope for #549).
#[test]
fn sweep_flags_loaded_plugin_when_trust_root_is_revoked() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let (sk, vk) = fixture_keypair(21);
        let key_id = "doomed-publisher";

        // Pre-populate the store with the key, then load a signed
        // plugin against it.
        let trust_store = TrustStore::new();
        trust_store.insert(key_id.to_string(), vk);
        let verifier = SignatureVerifier::new(trust_store.clone(), SignatureMode::Required);
        let (host, actor, _bus) = Host::new(
            PluginLoaderConfig {
                allow_unsigned: true,
                skip_wasm_validation: true,
                ..Default::default()
            },
            verifier,
            Blocklist::new(),
            mockforge_plugin_host::PlatformTrustStore::new(),
        );

        let outcome = tokio::select! {
            result = run_sweep_test(host, trust_store, sk, key_id) => result,
            _ = actor => panic!("actor exited before test body finished"),
        };
        outcome.expect("sweep test failed");
    });
}

async fn run_sweep_test(
    host: Host,
    trust_store: TrustStore,
    sk: SigningKey,
    key_id: &str,
) -> Result<(), String> {
    let sig = b64(&sk.sign(&build_signed_payload(MINIMAL_WASM, None)).to_bytes());
    host.load_plugin(
        "plug",
        "1.0.0",
        serde_json::json!({}),
        MINIMAL_WASM.to_vec(),
        Some(sig),
        Some(key_id.to_string()),
        None,
    )
    .await
    .map_err(|err| format!("initial load failed: {err}"))?;

    // Pre-sweep — nothing flagged.
    let pre = host.sweep_revoked_trust_roots().await.map_err(|err| err.to_string())?;
    if !pre.is_empty() {
        return Err(format!("pre-revocation sweep should be empty; got {:?}", pre));
    }

    // Simulate the refresh loop dropping the key out-of-band.
    let removed = trust_store.replace(std::collections::HashMap::new());
    if !removed.contains(&key_id.to_string()) {
        return Err(format!("replace() should report removed key; got {:?}", removed));
    }

    // Post-sweep — the plugin is flagged but still loaded.
    let flagged = host.sweep_revoked_trust_roots().await.map_err(|err| err.to_string())?;
    if !flagged.contains(&"plug".to_string()) {
        return Err(format!("post-revocation sweep should flag 'plug'; got {:?}", flagged));
    }
    let still_loaded = host.loaded_plugins().await.map_err(|err| err.to_string())?;
    if !still_loaded.iter().any(|(name, _)| name == "plug") {
        return Err("plug should still be loaded — hot-unload is out of scope per #549".into());
    }
    Ok(())
}

/// Poll `predicate` up to ~2 seconds. Avoids the flaky single-sleep
/// pattern (wiremock + tokio::time::interval wakeups have enough
/// jitter under CI load to make a single 100ms sleep unreliable).
async fn wait_for(mut predicate: impl FnMut() -> bool, what: &str) -> Result<(), String> {
    for _ in 0..100 {
        if predicate() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Err(format!("timed out waiting for {what}"))
}

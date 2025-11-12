//! Global SIEM emitter manager
//!
//! Provides a global SIEM emitter instance that can be accessed throughout the application
//! for emitting security events. Similar to the global request logger pattern.

use crate::security::siem::{SiemConfig, SiemEmitter};
use crate::Error;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Global SIEM emitter instance
static GLOBAL_SIEM_EMITTER: Lazy<Arc<RwLock<Option<SiemEmitter>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global SIEM emitter from configuration
///
/// This should be called once during application startup after loading the configuration.
/// If called multiple times, it will replace the existing emitter.
///
/// # Arguments
/// * `config` - SIEM configuration from ServerConfig
///
/// # Example
/// ```no_run
/// use mockforge_core::security::emitter::init_global_siem_emitter;
/// use mockforge_core::security::siem::SiemConfig;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SiemConfig::default();
/// init_global_siem_emitter(config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_global_siem_emitter(config: SiemConfig) -> Result<(), Error> {
    let emitter = SiemEmitter::from_config(config).await?;
    let mut global = GLOBAL_SIEM_EMITTER.write().await;
    *global = Some(emitter);
    debug!("Global SIEM emitter initialized");
    Ok(())
}

/// Get the global SIEM emitter instance (for internal use)
///
/// This function is primarily for internal use. Most code should use `emit_security_event`
/// instead, which handles the emitter access automatically.
///
/// Returns a reference to the emitter if initialized, None otherwise.
pub async fn get_global_siem_emitter() -> Option<Arc<RwLock<Option<SiemEmitter>>>> {
    Some(GLOBAL_SIEM_EMITTER.clone())
}

/// Emit a security event using the global SIEM emitter
///
/// This is a convenience function that automatically uses the global emitter if available.
/// If the emitter is not initialized or emission fails, errors are logged but not propagated.
///
/// # Arguments
/// * `event` - Security event to emit
///
/// # Example
/// ```no_run
/// use mockforge_core::security::emitter::emit_security_event;
/// use mockforge_core::security::events::{SecurityEvent, SecurityEventType, EventActor};
///
/// # async fn example() {
/// let event = SecurityEvent::new(SecurityEventType::AuthFailure, None, None)
///     .with_actor(EventActor {
///         user_id: None,
///         username: Some("admin".to_string()),
///         ip_address: Some("192.168.1.100".to_string()),
///         user_agent: Some("Mozilla/5.0".to_string()),
///     });
/// emit_security_event(event).await;
/// # }
/// ```
pub async fn emit_security_event(event: crate::security::events::SecurityEvent) {
    let global = GLOBAL_SIEM_EMITTER.read().await;
    if let Some(ref emitter) = *global {
        if let Err(e) = emitter.emit(event).await {
            error!("Failed to emit security event to SIEM: {}", e);
        }
    } else {
        debug!("SIEM emitter not initialized, skipping event emission");
    }
}

/// Emit a security event synchronously (spawns a task)
///
/// This is useful when you need to emit events from synchronous contexts.
/// The event is emitted in a background task.
///
/// # Arguments
/// * `event` - Security event to emit
pub fn emit_security_event_async(event: crate::security::events::SecurityEvent) {
    tokio::spawn(async move {
        emit_security_event(event).await;
    });
}

/// Check if SIEM emitter is initialized
pub async fn is_siem_emitter_initialized() -> bool {
    let global = GLOBAL_SIEM_EMITTER.read().await;
    global.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::events::SecurityEventType;
    use crate::security::siem::SiemConfig;

    #[tokio::test]
    async fn test_init_global_siem_emitter() {
        let config = SiemConfig::default();
        assert!(init_global_siem_emitter(config).await.is_ok());
        assert!(is_siem_emitter_initialized().await);
    }

    #[tokio::test]
    async fn test_get_global_siem_emitter() {
        let config = SiemConfig::default();
        init_global_siem_emitter(config).await.unwrap();

        let emitter_guard = get_global_siem_emitter().await;
        assert!(emitter_guard.is_some());
        let guard = emitter_guard.unwrap();
        let emitter = guard.read().await;
        assert!(emitter.is_some());
    }

    #[tokio::test]
    async fn test_emit_security_event() {
        let config = SiemConfig {
            enabled: false, // Disable to avoid actual network calls in tests
            protocol: None,
            destinations: vec![],
            filters: None,
        };
        init_global_siem_emitter(config).await.unwrap();

        let event = crate::security::events::SecurityEvent::new(
            SecurityEventType::AuthSuccess,
            None,
            None,
        );

        // Should not panic even with disabled emitter
        emit_security_event(event).await;
    }
}

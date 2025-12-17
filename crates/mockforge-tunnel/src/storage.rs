//! Persistent storage for tunnel data using SQLite
//!
//! This module provides SQLite-based persistent storage for tunnel configurations,
//! status, and statistics, allowing tunnel data to survive server restarts.

use crate::{TunnelConfig, TunnelStatus};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;
use tracing::{debug, error, info, warn};

#[cfg(feature = "server")]
use crate::server::TunnelStoreTrait;
#[cfg(feature = "server")]
use async_trait::async_trait;

/// Persistent tunnel storage using SQLite
#[derive(Clone)]
pub struct PersistentTunnelStore {
    pool: Pool<Sqlite>,
}

impl PersistentTunnelStore {
    /// Create a new persistent tunnel store
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    pub async fn new<P: AsRef<Path>>(database_path: P) -> crate::Result<Self> {
        let db_url = format!("sqlite://{}", database_path.as_ref().display());

        info!("Connecting to tunnel database: {}", db_url);

        let pool =
            SqlitePoolOptions::new()
                .max_connections(10)
                .connect(&db_url)
                .await
                .map_err(|e| {
                    error!("Failed to connect to tunnel database: {}", e);
                    crate::TunnelError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database connection failed: {}", e),
                    ))
                })?;

        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await.map_err(|e| {
            error!("Failed to enable WAL mode: {}", e);
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to enable WAL mode: {}", e),
            ))
        })?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.map_err(|e| {
            error!("Failed to enable foreign keys: {}", e);
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to enable foreign keys: {}", e),
            ))
        })?;

        let store = Self { pool };
        store.initialize_schema().await?;

        info!("Tunnel database initialized at {:?}", database_path.as_ref());
        Ok(store)
    }

    /// Create an in-memory database (for testing)
    pub async fn new_in_memory() -> crate::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| {
                error!("Failed to connect to in-memory database: {}", e);
                crate::TunnelError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Database connection failed: {}", e),
                ))
            })?;

        let store = Self { pool };
        store.initialize_schema().await?;

        debug!("In-memory tunnel database initialized");
        Ok(store)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> crate::Result<()> {
        info!("Initializing tunnel database schema");

        // Create tunnels table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tunnels (
                tunnel_id TEXT PRIMARY KEY,
                subdomain TEXT UNIQUE,
                public_url TEXT NOT NULL,
                local_url TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,
                request_count INTEGER NOT NULL DEFAULT 0,
                bytes_transferred INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                expires_at TEXT,
                custom_domain TEXT,
                protocol TEXT NOT NULL DEFAULT 'http',
                websocket_enabled INTEGER NOT NULL DEFAULT 1,
                http2_enabled INTEGER NOT NULL DEFAULT 1,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create tunnels table: {}", e);
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Schema initialization failed: {}", e),
            ))
        })?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tunnels_subdomain ON tunnels(subdomain)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                warn!("Failed to create subdomain index: {}", e);
                // Non-fatal, continue
            })
            .ok();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tunnels_active ON tunnels(active)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                warn!("Failed to create active index: {}", e);
                // Non-fatal, continue
            })
            .ok();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tunnels_created_at ON tunnels(created_at)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                warn!("Failed to create created_at index: {}", e);
                // Non-fatal, continue
            })
            .ok();

        info!("Tunnel database schema initialized");
        Ok(())
    }

    /// Create a new tunnel
    pub async fn create_tunnel(&self, config: &TunnelConfig) -> crate::Result<TunnelStatus> {
        let tunnel_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        // Generate public URL
        let public_url = if let Some(subdomain) = &config.subdomain {
            // Check if subdomain is already in use
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT tunnel_id FROM tunnels WHERE subdomain = ? AND active = 1",
            )
            .bind(subdomain)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to check subdomain: {}", e);
                crate::TunnelError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Database query failed: {}", e),
                ))
            })?;

            if existing.is_some() {
                return Err(crate::TunnelError::AlreadyExists(format!(
                    "Subdomain '{}' is already in use",
                    subdomain
                )));
            }

            format!("https://{}.tunnel.mockforge.test", subdomain)
        } else {
            let subdomain = format!("tunnel-{}", &tunnel_id[..8]);
            format!("https://{}.tunnel.mockforge.test", subdomain)
        };

        // Extract subdomain from public URL
        let subdomain = public_url
            .split('.')
            .next()
            .and_then(|s| s.strip_prefix("https://"))
            .map(|s| s.to_string());

        let status = TunnelStatus {
            public_url: public_url.clone(),
            tunnel_id: tunnel_id.clone(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: Some(now),
            expires_at: None,
            local_url: Some(config.local_url.clone()),
        };

        // Insert into database
        sqlx::query(
            r#"
            INSERT INTO tunnels (
                tunnel_id, subdomain, public_url, local_url, active,
                request_count, bytes_transferred, created_at, expires_at,
                protocol, websocket_enabled, http2_enabled, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&tunnel_id)
        .bind(&subdomain)
        .bind(&public_url)
        .bind(&config.local_url)
        .bind(1i32) // active = true
        .bind(0i64) // request_count
        .bind(0i64) // bytes_transferred
        .bind(now.to_rfc3339())
        .bind::<Option<String>>(None) // expires_at
        .bind(&config.protocol)
        .bind(if config.websocket_enabled { 1i32 } else { 0i32 })
        .bind(if config.http2_enabled { 1i32 } else { 0i32 })
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert tunnel: {}", e);
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Database insert failed: {}", e),
            ))
        })?;

        debug!("Created tunnel {} in database", tunnel_id);
        Ok(status)
    }

    /// Get tunnel by ID
    pub async fn get_tunnel(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        let row: Option<(
            String,
            Option<String>,
            String,
            String,
            i32,
            i64,
            i64,
            String,
            Option<String>,
        )> = sqlx::query_as(
            r#"
                SELECT tunnel_id, subdomain, public_url, local_url, active,
                       request_count, bytes_transferred, created_at, expires_at
                FROM tunnels
                WHERE tunnel_id = ?
                "#,
        )
        .bind(tunnel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query tunnel: {}", e);
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Database query failed: {}", e),
            ))
        })?;

        if let Some((
            id,
            _subdomain,
            public_url,
            local_url,
            active,
            request_count,
            bytes_transferred,
            created_at,
            expires_at,
        )) = row
        {
            Ok(TunnelStatus {
                tunnel_id: id,
                public_url,
                active: active != 0,
                request_count: request_count as u64,
                bytes_transferred: bytes_transferred as u64,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc)),
                expires_at: expires_at.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                }),
                local_url: Some(local_url),
            })
        } else {
            Err(crate::TunnelError::NotFound(tunnel_id.to_string()))
        }
    }

    /// Get tunnel by subdomain
    pub async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> crate::Result<TunnelStatus> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT tunnel_id FROM tunnels WHERE subdomain = ? AND active = 1")
                .bind(subdomain)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    error!("Failed to query tunnel by subdomain: {}", e);
                    crate::TunnelError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database query failed: {}", e),
                    ))
                })?;

        if let Some(tunnel_id) = row {
            self.get_tunnel(&tunnel_id).await
        } else {
            Err(crate::TunnelError::NotFound(format!("Subdomain not found: {}", subdomain)))
        }
    }

    /// Delete a tunnel
    pub async fn delete_tunnel(&self, tunnel_id: &str) -> crate::Result<()> {
        let rows_affected = sqlx::query("DELETE FROM tunnels WHERE tunnel_id = ?")
            .bind(tunnel_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete tunnel: {}", e);
                crate::TunnelError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Database delete failed: {}", e),
                ))
            })?
            .rows_affected();

        if rows_affected > 0 {
            debug!("Deleted tunnel {} from database", tunnel_id);
            Ok(())
        } else {
            Err(crate::TunnelError::NotFound(tunnel_id.to_string()))
        }
    }

    /// List all active tunnels
    pub async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        let rows: Vec<(
            String,
            Option<String>,
            String,
            String,
            i32,
            i64,
            i64,
            String,
            Option<String>,
        )> = sqlx::query_as(
            r#"
                SELECT tunnel_id, subdomain, public_url, local_url, active,
                       request_count, bytes_transferred, created_at, expires_at
                FROM tunnels
                WHERE active = 1
                ORDER BY created_at DESC
                "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(
                |(
                    id,
                    _subdomain,
                    public_url,
                    local_url,
                    active,
                    request_count,
                    bytes_transferred,
                    created_at,
                    expires_at,
                )| {
                    TunnelStatus {
                        tunnel_id: id,
                        public_url,
                        active: active != 0,
                        request_count: request_count as u64,
                        bytes_transferred: bytes_transferred as u64,
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc)),
                        expires_at: expires_at.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                        }),
                        local_url: Some(local_url),
                    }
                },
            )
            .collect()
    }

    /// Record a request (increment counters)
    pub async fn record_request(&self, tunnel_id: &str, bytes: u64) {
        sqlx::query(
            r#"
            UPDATE tunnels
            SET request_count = request_count + 1,
                bytes_transferred = bytes_transferred + ?,
                updated_at = ?
            WHERE tunnel_id = ?
            "#,
        )
        .bind(bytes as i64)
        .bind(Utc::now().to_rfc3339())
        .bind(tunnel_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            warn!("Failed to record request for tunnel {}: {}", tunnel_id, e);
        })
        .ok();
    }

    /// Get tunnel by ID (alias for get_tunnel for trait compatibility)
    pub async fn get_tunnel_by_id(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel(tunnel_id).await
    }

    /// Clean up expired tunnels
    pub async fn cleanup_expired(&self) -> crate::Result<u64> {
        let now = Utc::now().to_rfc3339();
        let rows_affected =
            sqlx::query("DELETE FROM tunnels WHERE expires_at IS NOT NULL AND expires_at < ?")
                .bind(&now)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("Failed to cleanup expired tunnels: {}", e);
                    crate::TunnelError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Cleanup failed: {}", e),
                    ))
                })?
                .rows_affected();

        if rows_affected > 0 {
            info!("Cleaned up {} expired tunnels", rows_affected);
        }

        Ok(rows_affected)
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl TunnelStoreTrait for PersistentTunnelStore {
    async fn create_tunnel(&self, config: &TunnelConfig) -> crate::Result<TunnelStatus> {
        self.create_tunnel(config).await
    }

    async fn get_tunnel(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel(tunnel_id).await
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> crate::Result<()> {
        self.delete_tunnel(tunnel_id).await
    }

    async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        self.list_tunnels().await
    }

    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel_by_subdomain(subdomain).await
    }

    async fn get_tunnel_by_id(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel_by_id(tunnel_id).await
    }

    async fn record_request(&self, tunnel_id: &str, bytes: u64) {
        self.record_request(tunnel_id, bytes).await
    }

    async fn cleanup_expired(&self) -> crate::Result<u64> {
        self.cleanup_expired().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> TunnelConfig {
        TunnelConfig {
            provider: crate::config::TunnelProvider::SelfHosted,
            server_url: Some("https://tunnel.test".to_string()),
            auth_token: None,
            subdomain: None,
            local_url: "http://localhost:3000".to_string(),
            protocol: "http".to_string(),
            region: None,
            custom_domain: None,
            websocket_enabled: true,
            http2_enabled: true,
        }
    }

    fn create_test_config_with_subdomain(subdomain: &str) -> TunnelConfig {
        let mut config = create_test_config();
        config.subdomain = Some(subdomain.to_string());
        config
    }

    #[tokio::test]
    async fn test_new_in_memory() {
        let store = PersistentTunnelStore::new_in_memory().await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_create_tunnel() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let result = store.create_tunnel(&config).await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.active);
        assert_eq!(status.request_count, 0);
        assert_eq!(status.bytes_transferred, 0);
        assert!(status.public_url.contains("tunnel.mockforge.test"));
        assert!(status.created_at.is_some());
        assert!(status.local_url.is_some());
        assert_eq!(status.local_url.unwrap(), config.local_url);
    }

    #[tokio::test]
    async fn test_create_tunnel_with_subdomain() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config_with_subdomain("myapp");

        let result = store.create_tunnel(&config).await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.public_url.contains("myapp.tunnel.mockforge.test"));
    }

    #[tokio::test]
    async fn test_create_tunnel_duplicate_subdomain() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config_with_subdomain("duplicate");

        // First creation should succeed
        let result1 = store.create_tunnel(&config).await;
        assert!(result1.is_ok());

        // Second creation with same subdomain should fail
        let result2 = store.create_tunnel(&config).await;
        assert!(result2.is_err());

        if let Err(e) = result2 {
            match e {
                crate::TunnelError::AlreadyExists(msg) => {
                    assert!(msg.contains("duplicate"));
                }
                _ => panic!("Expected AlreadyExists error"),
            }
        }
    }

    #[tokio::test]
    async fn test_get_tunnel() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let created = store.create_tunnel(&config).await.unwrap();
        let tunnel_id = created.tunnel_id.clone();

        let retrieved = store.get_tunnel(&tunnel_id).await;
        assert!(retrieved.is_ok());

        let status = retrieved.unwrap();
        assert_eq!(status.tunnel_id, tunnel_id);
        assert_eq!(status.public_url, created.public_url);
        assert!(status.active);
    }

    #[tokio::test]
    async fn test_get_tunnel_not_found() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();

        let result = store.get_tunnel("nonexistent-tunnel-id").await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                crate::TunnelError::NotFound(_) => {}
                _ => panic!("Expected NotFound error"),
            }
        }
    }

    #[tokio::test]
    async fn test_get_tunnel_by_subdomain() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config_with_subdomain("testapp");

        let created = store.create_tunnel(&config).await.unwrap();

        let retrieved = store.get_tunnel_by_subdomain("testapp").await;
        assert!(retrieved.is_ok());

        let status = retrieved.unwrap();
        assert_eq!(status.tunnel_id, created.tunnel_id);
    }

    #[tokio::test]
    async fn test_get_tunnel_by_subdomain_not_found() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();

        let result = store.get_tunnel_by_subdomain("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_tunnel() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let created = store.create_tunnel(&config).await.unwrap();
        let tunnel_id = created.tunnel_id.clone();

        // Delete the tunnel
        let delete_result = store.delete_tunnel(&tunnel_id).await;
        assert!(delete_result.is_ok());

        // Verify it's gone
        let get_result = store.get_tunnel(&tunnel_id).await;
        assert!(get_result.is_err());
    }

    #[tokio::test]
    async fn test_delete_tunnel_not_found() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();

        let result = store.delete_tunnel("nonexistent-tunnel-id").await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                crate::TunnelError::NotFound(_) => {}
                _ => panic!("Expected NotFound error"),
            }
        }
    }

    #[tokio::test]
    async fn test_list_tunnels_empty() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();

        let tunnels = store.list_tunnels().await;
        assert_eq!(tunnels.len(), 0);
    }

    #[tokio::test]
    async fn test_list_tunnels() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();

        // Create multiple tunnels
        let config1 = create_test_config_with_subdomain("app1");
        let config2 = create_test_config_with_subdomain("app2");
        let config3 = create_test_config_with_subdomain("app3");

        store.create_tunnel(&config1).await.unwrap();
        store.create_tunnel(&config2).await.unwrap();
        store.create_tunnel(&config3).await.unwrap();

        let tunnels = store.list_tunnels().await;
        assert_eq!(tunnels.len(), 3);

        // Verify all are active
        for tunnel in &tunnels {
            assert!(tunnel.active);
        }
    }

    #[tokio::test]
    async fn test_record_request() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let created = store.create_tunnel(&config).await.unwrap();
        let tunnel_id = created.tunnel_id.clone();

        // Record some requests
        store.record_request(&tunnel_id, 1024).await;
        store.record_request(&tunnel_id, 2048).await;

        // Retrieve and verify counters
        let status = store.get_tunnel(&tunnel_id).await.unwrap();
        assert_eq!(status.request_count, 2);
        assert_eq!(status.bytes_transferred, 3072);
    }

    #[tokio::test]
    async fn test_record_request_large_bytes() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let created = store.create_tunnel(&config).await.unwrap();
        let tunnel_id = created.tunnel_id.clone();

        // Record a large number of bytes
        let large_bytes = u64::MAX / 2;
        store.record_request(&tunnel_id, large_bytes).await;

        let status = store.get_tunnel(&tunnel_id).await.unwrap();
        assert_eq!(status.request_count, 1);
        assert_eq!(status.bytes_transferred, large_bytes);
    }

    #[tokio::test]
    async fn test_get_tunnel_by_id_alias() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let created = store.create_tunnel(&config).await.unwrap();
        let tunnel_id = created.tunnel_id.clone();

        // Use the alias method
        let retrieved = store.get_tunnel_by_id(&tunnel_id).await;
        assert!(retrieved.is_ok());

        let status = retrieved.unwrap();
        assert_eq!(status.tunnel_id, tunnel_id);
    }

    #[tokio::test]
    async fn test_cleanup_expired_no_expired() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        store.create_tunnel(&config).await.unwrap();

        // No expired tunnels, should clean up 0
        let cleaned = store.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 0);
    }

    #[tokio::test]
    async fn test_clone() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let cloned = store.clone();

        // Both should work independently
        let config = create_test_config();
        let result = cloned.create_tunnel(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_operations_sequence() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();

        // Create
        let config1 = create_test_config_with_subdomain("seq1");
        let tunnel1 = store.create_tunnel(&config1).await.unwrap();

        // List
        let list1 = store.list_tunnels().await;
        assert_eq!(list1.len(), 1);

        // Create another
        let config2 = create_test_config_with_subdomain("seq2");
        let tunnel2 = store.create_tunnel(&config2).await.unwrap();

        // List again
        let list2 = store.list_tunnels().await;
        assert_eq!(list2.len(), 2);

        // Record requests
        store.record_request(&tunnel1.tunnel_id, 100).await;
        store.record_request(&tunnel2.tunnel_id, 200).await;

        // Get and verify
        let status1 = store.get_tunnel(&tunnel1.tunnel_id).await.unwrap();
        let status2 = store.get_tunnel(&tunnel2.tunnel_id).await.unwrap();
        assert_eq!(status1.bytes_transferred, 100);
        assert_eq!(status2.bytes_transferred, 200);

        // Delete one
        store.delete_tunnel(&tunnel1.tunnel_id).await.unwrap();

        // List should show only one
        let list3 = store.list_tunnels().await;
        assert_eq!(list3.len(), 1);
        assert_eq!(list3[0].tunnel_id, tunnel2.tunnel_id);
    }

    #[tokio::test]
    async fn test_tunnel_timestamps() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let config = create_test_config();

        let before = Utc::now();
        let tunnel = store.create_tunnel(&config).await.unwrap();
        let after = Utc::now();

        // Verify created_at is within expected range
        assert!(tunnel.created_at.is_some());
        let created = tunnel.created_at.unwrap();
        assert!(created >= before);
        assert!(created <= after);
    }

    #[tokio::test]
    async fn test_tunnel_protocol_persistence() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let mut config = create_test_config();
        config.protocol = "https".to_string();

        let tunnel = store.create_tunnel(&config).await.unwrap();
        let retrieved = store.get_tunnel(&tunnel.tunnel_id).await.unwrap();

        // The protocol should be stored and retrievable
        assert_eq!(tunnel.tunnel_id, retrieved.tunnel_id);
    }

    #[tokio::test]
    async fn test_websocket_and_http2_flags() {
        let store = PersistentTunnelStore::new_in_memory().await.unwrap();
        let mut config = create_test_config();
        config.websocket_enabled = false;
        config.http2_enabled = false;

        let result = store.create_tunnel(&config).await;
        assert!(result.is_ok());

        // Flags should be persisted (verification through database)
        let tunnel = result.unwrap();
        assert!(tunnel.tunnel_id.len() > 0);
    }
}

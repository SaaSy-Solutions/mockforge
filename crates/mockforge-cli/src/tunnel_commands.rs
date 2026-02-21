//! Tunnel commands for exposing local servers via public URLs

use mockforge_tunnel::{TunnelConfig, TunnelManager, TunnelProvider};
use tracing::info;

#[derive(clap::Subcommand)]
pub enum TunnelSubcommand {
    /// Start a tunnel to expose local server via public URL
    ///
    /// Examples:
    ///   mockforge tunnel start --local-url http://localhost:3000
    ///   mockforge tunnel start --local-url http://localhost:3000 --subdomain my-api
    ///   mockforge tunnel start --local-url http://localhost:3000 --provider cloud
    Start {
        /// Local server URL to tunnel
        #[arg(long, default_value = "http://localhost:3000")]
        local_url: String,

        /// Tunnel provider (self, cloud, cloudflare)
        #[arg(long, default_value = "self")]
        provider: String,

        /// Tunnel server URL (for self-hosted provider)
        #[arg(long)]
        server_url: Option<String>,

        /// Authentication token (if required)
        #[arg(long)]
        auth_token: Option<String>,

        /// Request a specific subdomain (if available)
        #[arg(long)]
        subdomain: Option<String>,

        /// Custom domain (if provider supports it)
        #[arg(long)]
        custom_domain: Option<String>,

        /// Protocol (http, https, ws, wss)
        #[arg(long, default_value = "http")]
        protocol: String,

        /// Region (if provider supports it)
        #[arg(long)]
        region: Option<String>,

        /// Enable WebSocket support
        #[arg(long, default_value = "true")]
        websocket_enabled: bool,
    },

    /// Stop the active tunnel
    Stop {
        /// Tunnel server URL (for self-hosted provider)
        #[arg(long)]
        server_url: Option<String>,

        /// Authentication token (if required)
        #[arg(long)]
        auth_token: Option<String>,

        /// Tunnel ID (if not specified, stops active tunnel)
        #[arg(long)]
        tunnel_id: Option<String>,
    },

    /// Get tunnel status
    Status {
        /// Tunnel server URL (for self-hosted provider)
        #[arg(long)]
        server_url: Option<String>,

        /// Authentication token (if required)
        #[arg(long)]
        auth_token: Option<String>,

        /// Tunnel ID (if not specified, shows active tunnel)
        #[arg(long)]
        tunnel_id: Option<String>,
    },

    /// List all active tunnels
    List {
        /// Tunnel server URL (for self-hosted provider)
        #[arg(long)]
        server_url: Option<String>,

        /// Authentication token (if required)
        #[arg(long)]
        auth_token: Option<String>,
    },
}

pub async fn handle_tunnel_command(cmd: TunnelSubcommand) -> anyhow::Result<()> {
    match cmd {
        TunnelSubcommand::Start {
            local_url,
            provider,
            server_url,
            auth_token,
            subdomain,
            custom_domain,
            protocol,
            region,
            websocket_enabled,
        } => {
            let provider_enum = match provider.as_str() {
                "self" => TunnelProvider::SelfHosted,
                "cloud" => TunnelProvider::Cloud,
                "cloudflare" => TunnelProvider::Cloudflare,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unknown provider: {}. Supported: self, cloud, cloudflare",
                        provider
                    ));
                }
            };

            let mut config = TunnelConfig::new(&local_url).with_provider(provider_enum);

            if let Some(token) = auth_token {
                config.auth_token = Some(token);
            }

            if let Some(subdomain) = subdomain {
                config.subdomain = Some(subdomain);
            }
            if let Some(custom_domain) = custom_domain {
                config.custom_domain = Some(custom_domain);
            }
            if let Some(region) = region {
                config.region = Some(region);
            }
            config.protocol = protocol;
            config.websocket_enabled = websocket_enabled;

            // For self-hosted, require server_url
            if matches!(config.provider, TunnelProvider::SelfHosted) {
                config.server_url = Some(
                    server_url
                        .or_else(|| std::env::var("MOCKFORGE_TUNNEL_SERVER_URL").ok())
                        .ok_or_else(|| {
                            anyhow::anyhow!("server_url required for self-hosted provider. Set via --server-url or MOCKFORGE_TUNNEL_SERVER_URL env var")
                        })?
                );
            }

            let manager = TunnelManager::new(&config)?;

            info!("Creating tunnel to {}...", local_url);
            let status = manager.create_tunnel(&config).await?;

            println!("✅ Tunnel created successfully!");
            println!("   Public URL: {}", status.public_url);
            println!("   Tunnel ID: {}", status.tunnel_id);
            println!("   Status: {}", if status.active { "Active" } else { "Inactive" });

            if let Some(expires_at) = status.expires_at {
                println!("   Expires at: {}", expires_at);
            }

            Ok(())
        }

        TunnelSubcommand::Stop {
            server_url,
            auth_token,
            tunnel_id,
        } => {
            let mut config = TunnelConfig::default();
            config.server_url =
                server_url.or_else(|| std::env::var("MOCKFORGE_TUNNEL_SERVER_URL").ok());

            if config.server_url.is_none() {
                return Err(anyhow::anyhow!("server_url required. Set via --server-url or MOCKFORGE_TUNNEL_SERVER_URL env var"));
            }

            if let Some(token) = auth_token {
                config.auth_token = Some(token);
            }

            let manager = TunnelManager::new(&config)?;

            if let Some(tunnel_id) = tunnel_id {
                // Stop specific tunnel by ID
                info!("Stopping tunnel: {}", tunnel_id);
                manager.stop_tunnel_by_id(&tunnel_id).await?;
                println!("✅ Tunnel {} stopped successfully", tunnel_id);
                Ok(())
            } else {
                // Stop active tunnel
                info!("Stopping active tunnel...");
                manager.stop_tunnel().await?;
                println!("✅ Tunnel stopped successfully");
                Ok(())
            }
        }

        TunnelSubcommand::Status {
            server_url,
            auth_token,
            tunnel_id,
        } => {
            let mut config = TunnelConfig::default();
            config.server_url =
                server_url.or_else(|| std::env::var("MOCKFORGE_TUNNEL_SERVER_URL").ok());

            if config.server_url.is_none() {
                return Err(anyhow::anyhow!("server_url required. Set via --server-url or MOCKFORGE_TUNNEL_SERVER_URL env var"));
            }

            if let Some(token) = auth_token {
                config.auth_token = Some(token);
            }

            let manager = TunnelManager::new(&config)?;

            if let Some(tunnel_id) = tunnel_id {
                // Get specific tunnel status
                let status = manager
                    .list_tunnels()
                    .await?
                    .into_iter()
                    .find(|t| t.tunnel_id == tunnel_id)
                    .ok_or_else(|| anyhow::anyhow!("Tunnel not found: {}", tunnel_id))?;

                print_tunnel_status(&status);
            } else {
                // Get active tunnel status
                if let Some(status) = manager.get_status().await? {
                    print_tunnel_status(&status);
                } else {
                    println!("No active tunnel found");
                }
            }

            Ok(())
        }

        TunnelSubcommand::List {
            server_url,
            auth_token,
        } => {
            let mut config = TunnelConfig::default();
            config.server_url =
                server_url.or_else(|| std::env::var("MOCKFORGE_TUNNEL_SERVER_URL").ok());

            if config.server_url.is_none() {
                return Err(anyhow::anyhow!("server_url required. Set via --server-url or MOCKFORGE_TUNNEL_SERVER_URL env var"));
            }

            if let Some(token) = auth_token {
                config.auth_token = Some(token);
            }

            let manager = TunnelManager::new(&config)?;
            let tunnels = manager.list_tunnels().await?;

            if tunnels.is_empty() {
                println!("No active tunnels found");
            } else {
                println!("Active tunnels ({})", tunnels.len());
                println!();
                for tunnel in tunnels {
                    print_tunnel_status(&tunnel);
                    println!();
                }
            }

            Ok(())
        }
    }
}

fn print_tunnel_status(status: &mockforge_tunnel::TunnelStatus) {
    println!("Tunnel ID: {}", status.tunnel_id);
    println!("  Public URL: {}", status.public_url);
    println!("  Status: {}", if status.active { "Active" } else { "Inactive" });
    println!("  Requests: {}", status.request_count);
    println!("  Bytes: {}", status.bytes_transferred);
    if let Some(created_at) = status.created_at {
        println!("  Created: {}", created_at);
    }
    if let Some(expires_at) = status.expires_at {
        println!("  Expires: {}", expires_at);
    }
}

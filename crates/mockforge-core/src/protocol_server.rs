//! Protocol server lifecycle trait for uniform server startup and shutdown.
//!
//! This module provides the [`MockProtocolServer`] trait, which abstracts the
//! lifecycle management of mock protocol servers. Each protocol crate (gRPC, FTP,
//! TCP, SMTP, etc.) can implement this trait to provide a uniform startup interface,
//! enabling the CLI to launch all protocols through a single, consistent code path.
//!
//! # Design
//!
//! The trait is intentionally minimal — it covers the core lifecycle operations
//! (start, shutdown, identification) without imposing protocol-specific details.
//! Protocol crates wrap their existing server startup logic in a struct that
//! implements this trait; the actual server code remains unchanged.
//!
//! # Example
//!
//! ```rust,no_run
//! use mockforge_core::protocol_server::MockProtocolServer;
//! use mockforge_core::protocol_abstraction::Protocol;
//! use async_trait::async_trait;
//!
//! struct MyServer { port: u16 }
//!
//! #[async_trait]
//! impl MockProtocolServer for MyServer {
//!     fn protocol(&self) -> Protocol { Protocol::Tcp }
//!     async fn start(
//!         &self,
//!         shutdown: tokio::sync::watch::Receiver<()>,
//!     ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         // Run until shutdown signal
//!         let _ = shutdown;
//!         Ok(())
//!     }
//!     fn port(&self) -> u16 { self.port }
//!     fn description(&self) -> String {
//!         format!("TCP server on port {}", self.port)
//!     }
//! }
//! ```

use crate::protocol_abstraction::Protocol;
use async_trait::async_trait;

/// Trait for mock protocol server lifecycle management.
///
/// Each protocol crate implements this to provide a uniform startup interface.
/// The CLI can collect `Box<dyn MockProtocolServer>` instances and launch them
/// all through a single code path, rather than having bespoke startup logic
/// for each protocol.
///
/// Implementations should:
/// - Wrap existing server startup code (not rewrite it)
/// - Run until the shutdown signal is received in [`start`](MockProtocolServer::start)
/// - Return errors from [`start`](MockProtocolServer::start) if the server fails to bind or encounters a fatal error
#[async_trait]
pub trait MockProtocolServer: Send + Sync {
    /// Which protocol this server handles.
    fn protocol(&self) -> Protocol;

    /// Start the server, running until the shutdown signal is received.
    ///
    /// The server should listen on its configured address and handle requests
    /// until the `shutdown` receiver signals (i.e., the sender is dropped or
    /// a value is sent). Implementations should use `tokio::select!` to
    /// combine the server's accept loop with the shutdown signal.
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to bind, encounters a fatal I/O
    /// error, or any other unrecoverable condition.
    async fn start(
        &self,
        shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// The port this server is listening on.
    fn port(&self) -> u16;

    /// Human-readable description for logging (e.g., "gRPC server on port 50051").
    fn description(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyServer {
        port: u16,
    }

    #[async_trait]
    impl MockProtocolServer for DummyServer {
        fn protocol(&self) -> Protocol {
            Protocol::Tcp
        }

        async fn start(
            &self,
            mut shutdown: tokio::sync::watch::Receiver<()>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Wait for shutdown
            let _ = shutdown.changed().await;
            Ok(())
        }

        fn port(&self) -> u16 {
            self.port
        }

        fn description(&self) -> String {
            format!("Dummy TCP server on port {}", self.port)
        }
    }

    #[test]
    fn test_protocol_server_trait_object() {
        let server: Box<dyn MockProtocolServer> = Box::new(DummyServer { port: 9999 });
        assert_eq!(server.protocol(), Protocol::Tcp);
        assert_eq!(server.port(), 9999);
        assert_eq!(server.description(), "Dummy TCP server on port 9999");
    }

    #[tokio::test]
    async fn test_protocol_server_shutdown() {
        let server = DummyServer { port: 8080 };
        let (tx, rx) = tokio::sync::watch::channel(());

        let handle = tokio::spawn(async move { server.start(rx).await });

        // Signal shutdown
        drop(tx);

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_protocol_server_description() {
        let server = DummyServer { port: 50051 };
        assert!(server.description().contains("50051"));
    }
}

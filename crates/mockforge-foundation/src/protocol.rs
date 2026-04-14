//! Protocol type enumeration for multi-protocol support
//!
//! The canonical `Protocol` enum. `mockforge-core::protocol_abstraction` and
//! `mockforge-contracts::protocol` both re-export from here so cross-crate
//! types can interoperate.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Protocol type enumeration for multi-protocol support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Protocol {
    /// HTTP/REST protocol for RESTful APIs
    Http,
    /// GraphQL protocol for GraphQL APIs
    GraphQL,
    /// gRPC protocol for gRPC services
    Grpc,
    /// WebSocket protocol for real-time bidirectional communication
    WebSocket,
    /// SMTP/Email protocol for email communication
    Smtp,
    /// MQTT protocol for IoT messaging and pub/sub
    Mqtt,
    /// FTP protocol for file transfer operations
    Ftp,
    /// Kafka protocol for distributed event streaming
    Kafka,
    /// RabbitMQ/AMQP protocol for message queuing
    RabbitMq,
    /// AMQP protocol for advanced message queuing scenarios
    Amqp,
    /// TCP protocol for raw TCP connections
    Tcp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Http => write!(f, "HTTP"),
            Protocol::GraphQL => write!(f, "GraphQL"),
            Protocol::Grpc => write!(f, "gRPC"),
            Protocol::WebSocket => write!(f, "WebSocket"),
            Protocol::Smtp => write!(f, "SMTP"),
            Protocol::Mqtt => write!(f, "MQTT"),
            Protocol::Ftp => write!(f, "FTP"),
            Protocol::Kafka => write!(f, "Kafka"),
            Protocol::RabbitMq => write!(f, "RabbitMQ"),
            Protocol::Amqp => write!(f, "AMQP"),
            Protocol::Tcp => write!(f, "TCP"),
        }
    }
}

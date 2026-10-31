//! Fly services for hosted-mock machines.

use crate::deployment::flyio::{FlyioPort, FlyioService};
use crate::models::HostedMock;

/// Fly services for a hosted mock: the HTTP listener (which also serves
/// WebSocket upgrades and /graphql) plus one TCP service per extra protocol.
/// Machines stop when idle and Fly starts them again on the next connection,
/// so an unused mock costs nothing while stopped.
pub(crate) fn fly_services(deployment: &HostedMock) -> Vec<FlyioService> {
    let idle_stop = |internal_port: u16, ports: Vec<FlyioPort>| FlyioService {
        protocol: "tcp".to_string(),
        internal_port,
        ports,
        autostop: Some("stop".to_string()),
        autostart: Some(true),
        min_machines_running: Some(0),
    };
    let mut services = vec![idle_stop(
        3000,
        vec![
            FlyioPort {
                port: 80,
                handlers: vec!["http".to_string()],
            },
            FlyioPort {
                port: 443,
                handlers: vec!["tls".to_string(), "http".to_string()],
            },
        ],
    )];
    let mut bound_internal_ports = std::collections::HashSet::from([3000]);
    for protocol in deployment.enabled_protocols() {
        let Some(internal) = protocol.internal_port() else {
            continue; // HTTP / WS / GraphQL share 3000.
        };
        if !bound_internal_ports.insert(internal) {
            continue;
        }
        let public = protocol.public_port().unwrap_or(internal);
        let handlers = protocol.fly_handlers().iter().map(|s| (*s).to_string()).collect();
        services.push(idle_stop(
            internal,
            vec![FlyioPort {
                port: public,
                handlers,
            }],
        ));
    }
    services
}

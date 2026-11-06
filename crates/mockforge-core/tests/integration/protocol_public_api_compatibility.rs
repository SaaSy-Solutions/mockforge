//! Public compatibility between core's historical paths and the protocol owner.

use mockforge_core::protocol_abstraction::AuthMiddleware;
use mockforge_protocol::{MiddlewareAction, ProtocolMiddleware, ResponseStatus};
use std::any::TypeId;

#[test]
fn protocol_requests_cross_public_paths_without_conversion() {
    let protocol = mockforge_foundation::protocol::Protocol::Tcp;
    let request = mockforge_core::ProtocolRequest {
        protocol,
        path: "/compatibility".to_string(),
        ..Default::default()
    };
    let request: mockforge_core::protocol_abstraction::ProtocolRequest = request;
    let request: mockforge_protocol::protocol_abstraction::ProtocolRequest = request;
    let request: mockforge_protocol::ProtocolRequest = request;
    let protocol: mockforge_core::Protocol = request.protocol;
    let protocol: mockforge_protocol::Protocol = protocol;

    assert_eq!(protocol, mockforge_foundation::protocol::Protocol::Tcp);
    assert_eq!(request.path, "/compatibility");
}

#[test]
fn errors_and_results_keep_foundation_identity() {
    let error = mockforge_foundation::Error::Generic("compatibility".to_string());
    let error: mockforge_protocol::Error = error;
    let error: mockforge_core::Error = error;
    let result: mockforge_core::Result<()> = Err(error);
    let result: mockforge_protocol::Result<()> = result;
    let result: mockforge_foundation::Result<()> = result;

    assert!(matches!(result, Err(mockforge_foundation::Error::Generic(message))
        if message == "compatibility"));
}

#[test]
fn fixture_options_keep_the_historical_core_path() {
    let options = mockforge_core::fixture_store::FixtureLoadOptions::yaml_array_strict();
    let options: mockforge_protocol::fixture_store::FixtureLoadOptions = options;
    let mode: mockforge_core::fixture_store::FixtureLoadErrorMode = options.error_mode;

    assert_eq!(mode, mockforge_protocol::fixture_store::FixtureLoadErrorMode::FailFast);
    assert_eq!(
        options.granularity,
        mockforge_core::fixture_store::FixtureFileGranularity::Array
    );
}

#[test]
fn server_trait_keeps_the_historical_core_path() {
    assert_eq!(
        TypeId::of::<dyn mockforge_core::protocol_server::MockProtocolServer>(),
        TypeId::of::<dyn mockforge_protocol::protocol_server::MockProtocolServer>(),
    );
}

#[tokio::test]
async fn core_auth_adapter_accepts_the_protocol_owner_request() {
    let adapter = AuthMiddleware::new(mockforge_core::config::AuthConfig {
        require_auth: true,
        jwt: None,
        api_key: None,
        oauth2: None,
        basic_auth: None,
    });
    let middleware: &dyn ProtocolMiddleware = &adapter;
    let mut request = mockforge_protocol::ProtocolRequest {
        path: "/health".to_string(),
        ..Default::default()
    };

    assert_eq!(middleware.name(), "AuthMiddleware");
    assert!(matches!(
        middleware.process_request(&mut request).await,
        Ok(MiddlewareAction::Continue)
    ));

    request.path = "/private".to_string();
    let action = middleware
        .process_request(&mut request)
        .await
        .expect("auth middleware response");
    let MiddlewareAction::ShortCircuit(response) = action else {
        panic!("protected requests without credentials must be denied");
    };
    assert!(matches!(response.status, ResponseStatus::HttpStatus(401)));
}

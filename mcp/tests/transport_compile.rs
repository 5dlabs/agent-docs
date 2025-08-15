//! Compile-surface test for transport module (MVP)

use mcp::server::McpServerState;
use mcp::transport::{
    unified_mcp_handler, McpSession, SessionManager, TransportConfig, TransportError,
};
use std::time::Duration;

#[test]
fn test_transport_config_compiles() {
    // Test that TransportConfig can be created and has expected fields
    let config = TransportConfig {
        protocol_version: "2025-06-18".to_string(),
        session_timeout: Duration::from_secs(300),
        heartbeat_interval: Duration::from_secs(30),
        max_json_body_bytes: 2 * 1024 * 1024,
    };

    assert_eq!(config.protocol_version, "2025-06-18");
    assert_eq!(config.session_timeout, Duration::from_secs(300));
    assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
    assert_eq!(config.max_json_body_bytes, 2 * 1024 * 1024);
}

#[test]
fn test_transport_config_default() {
    // Test that TransportConfig implements Default
    let config = TransportConfig::default();
    assert_eq!(config.protocol_version, "2025-06-18");
    assert_eq!(config.session_timeout, Duration::from_secs(300));
    assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
    assert_eq!(config.max_json_body_bytes, 2 * 1024 * 1024);
}

#[test]
fn test_transport_error_variants_exist() {
    // Test that all expected error variants exist and can be constructed
    let method_error = TransportError::MethodNotAllowed;
    assert_eq!(method_error.to_string(), "Method not allowed");
    let _protocol_error = TransportError::UnsupportedProtocolVersion("invalid".to_string());
    let _session_error = TransportError::SessionNotFound(uuid::Uuid::new_v4());
    let _invalid_session = TransportError::InvalidSessionId("invalid".to_string());
    let lock_error = TransportError::SessionLockError;
    assert_eq!(lock_error.to_string(), "Session lock error");
    let content_error = TransportError::MissingContentType;
    assert_eq!(content_error.to_string(), "Missing content type");
    let _invalid_content = TransportError::InvalidContentType("text/plain".to_string());
    let _json_error = TransportError::JsonParseError("invalid json".to_string());
    let _internal_error = TransportError::InternalError("internal error".to_string());

    // Verify error messages
    assert_eq!(
        TransportError::MethodNotAllowed.to_string(),
        "Method not allowed"
    );
    assert_eq!(
        TransportError::UnsupportedProtocolVersion("test".to_string()).to_string(),
        "Unsupported protocol version: test"
    );
}

#[test]
fn test_session_management_types() {
    // Test that session management types can be created
    let config = TransportConfig::default();
    let _session_manager = SessionManager::new(config);

    // Test session creation
    let session = McpSession::new();
    assert!(!session.is_expired(Duration::from_secs(1)));

    // Test session ID type
    let _ = session.id;
}

#[test]
fn test_transport_api_surface() {
    // This test verifies that all expected exports from the transport module are available

    // Types
    let config = TransportConfig::default();
    let _session_manager = SessionManager::new(config);
    let _ = (
        TransportError::MethodNotAllowed,
        None::<McpServerState>,
        unified_mcp_handler,
        McpSession::new().id,
    );
    // If we get here without compile errors, the API surface is correct
}

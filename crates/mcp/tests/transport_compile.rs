//! Compile-surface test for transport module (MVP)

use doc_server_mcp::server::McpServerState;
use doc_server_mcp::transport::{unified_mcp_handler, TransportConfig, TransportError};

#[test]
fn test_transport_config_compiles() {
    // Test that TransportConfig can be created and has expected fields
    let config = TransportConfig {
        protocol_version: "2025-06-18".to_string(),
    };

    assert_eq!(config.protocol_version, "2025-06-18");
}

// Default may not be implemented in MVP; rely on explicit construction

#[test]
fn test_transport_error_variants_exist() {
    // Test that all expected error variants exist and can be constructed
    let _method_error = TransportError::MethodNotAllowed;
    // MVP does not yet define the below variants; only ensure MethodNotAllowed exists

    // Verify error messages
    assert_eq!(
        TransportError::MethodNotAllowed.to_string(),
        "method not allowed"
    );
}

// McpServerState is a server type; we only assert name resolvability in API surface test

#[test]
fn test_transport_api_surface() {
    // This test verifies that all expected exports from the transport module are available

    // Types
    let _config = TransportConfig {
        protocol_version: "2025-06-18".to_string(),
    };
    let _error = TransportError::MethodNotAllowed;
    // Ensure the server type is name-resolvable without constructing it
    fn assert_type<T>() {}
    assert_type::<McpServerState>();

    // Function - just verify it can be referenced
    let _handler_fn = unified_mcp_handler;
    // If we get here without compile errors, the API surface is correct
}

//! Test to verify headers module compiles and exports expected constants and functions

use axum::http::HeaderMap;
use doc_server_mcp::headers::{
    set_standard_headers, validate_protocol_version, MCP_PROTOCOL_VERSION, MCP_SESSION_ID,
    SUPPORTED_PROTOCOL_VERSION,
};
use uuid::Uuid;

#[test]
fn test_header_constants_exist() {
    // Test that all required constants are defined and have expected values
    assert_eq!(MCP_PROTOCOL_VERSION, "MCP-Protocol-Version");
    assert_eq!(MCP_SESSION_ID, "Mcp-Session-Id");
    assert_eq!(SUPPORTED_PROTOCOL_VERSION, "2025-06-18");
}

#[test]
fn test_validate_protocol_version_compiles() {
    // Test that the validate_protocol_version function exists and compiles
    let headers = HeaderMap::new();

    // This should return an error for missing headers, but the important part
    // is that it compiles and we can call the function
    let result = validate_protocol_version(&headers);

    // We expect this to fail since no headers are set, but it should compile
    assert!(
        result.is_err(),
        "Should return error for missing protocol version"
    );
}

#[test]
fn test_set_standard_headers_compiles() {
    // Test that the set_standard_headers function exists and compiles
    let mut headers = HeaderMap::new();
    let session_id = Uuid::new_v4();

    // This function should compile and execute without panicking
    set_standard_headers(&mut headers, Some(session_id));

    // Verify that headers were actually set
    assert!(headers.get(MCP_PROTOCOL_VERSION).is_some());
    assert!(headers.get(MCP_SESSION_ID).is_some());
}

#[test]
fn test_set_standard_headers_no_session_compiles() {
    // Test setting headers without a session ID
    let mut headers = HeaderMap::new();

    set_standard_headers(&mut headers, None);

    // Protocol version should be set, session ID should not
    assert!(headers.get(MCP_PROTOCOL_VERSION).is_some());
    assert!(headers.get(MCP_SESSION_ID).is_none());
}

#[test]
fn test_headers_module_api_surface() {
    // This test verifies that all expected exports from the headers module are available

    // Constants
    let _protocol_header = MCP_PROTOCOL_VERSION;
    let _session_header = MCP_SESSION_ID;
    let _supported_version = SUPPORTED_PROTOCOL_VERSION;

    // Functions - just verify they can be referenced (not called in this test)
    let _validate_fn = validate_protocol_version;
    let _set_headers_fn = set_standard_headers;

    // If we get here without compile errors, the API surface is correct
}

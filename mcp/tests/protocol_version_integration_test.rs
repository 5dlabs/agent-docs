//! Integration tests for Protocol Version Negotiation and Headers
//!
//! This test suite verifies the end-to-end functionality of protocol version
//! handling, header validation, session management, and response formatting
//! for the fixed MCP protocol version "2025-06-18".

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use mcp::{
    headers::{
        set_json_response_headers, set_standard_headers, validate_protocol_version,
        McpProtocolVersionHeader, ProtocolVersionError, CONTENT_TYPE_JSON, MCP_PROTOCOL_VERSION,
        MCP_SESSION_ID, SUPPORTED_PROTOCOL_VERSION,
    },
    protocol_version::{
        ProtocolRegistry, ProtocolVersion, SUPPORTED_PROTOCOL_VERSION as PROTOCOL_SUPPORTED_VERSION,
    },
    session::{ClientInfo, SessionConfig, SessionManager},
};
use serde_json::json;
use uuid::Uuid;

/// Test protocol version registry integration
#[test]
fn test_protocol_version_registry_integration() {
    let registry = ProtocolRegistry::new();

    // Test current version
    assert_eq!(registry.current_version(), ProtocolVersion::V2025_06_18);
    assert_eq!(registry.current_version_string(), "2025-06-18");

    // Test version validation
    assert!(registry.is_version_string_supported("2025-06-18"));
    assert!(!registry.is_version_string_supported("2024-11-05"));
    assert!(!registry.is_version_string_supported("invalid-version"));

    // Test version string validation
    assert!(registry.validate_version_string("2025-06-18").is_ok());
    assert!(registry.validate_version_string("2024-11-05").is_err());
}

/// Test header validation with protocol version registry
#[test]
fn test_header_validation_with_registry() {
    let mut headers = HeaderMap::new();

    // Test with supported version
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2025-06-18"));
    assert!(validate_protocol_version(&headers).is_ok());

    // Test with unsupported version
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2024-11-05"));
    assert_eq!(
        validate_protocol_version(&headers),
        Err(StatusCode::BAD_REQUEST)
    );

    // Test with missing header
    headers.clear();
    assert_eq!(
        validate_protocol_version(&headers),
        Err(StatusCode::BAD_REQUEST)
    );
}

/// Test session creation with protocol version consistency
#[test]
fn test_session_protocol_version_consistency() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    // Create session and verify protocol version
    let session_id = manager.create_session(None).unwrap();
    let session = manager.get_session(session_id).unwrap();

    assert_eq!(session.protocol_version, "2025-06-18");
    assert!(session.is_protocol_version_supported());

    // Validate protocol version via manager
    assert!(manager
        .validate_session_protocol_version(session_id, "2025-06-18")
        .is_ok());
    assert!(manager
        .validate_session_protocol_version(session_id, "2024-11-05")
        .is_err());
}

/// Test initialize handler response structure (unit test for static response)
#[test]
fn test_initialize_response_structure() {
    // Test the structure of an initialize response using the protocol registry
    let registry = ProtocolRegistry::new();

    // Simulate the initialize response structure
    let initialize_response = json!({
        "protocolVersion": registry.current_version_string(),
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "mcp",
            "version": "0.1.0"  // This would be env!("CARGO_PKG_VERSION") in actual handler
        }
    });

    // Verify the response contains the correct protocol version
    assert_eq!(initialize_response["protocolVersion"], "2025-06-18");

    // Verify server info structure
    assert!(initialize_response["serverInfo"].is_object());
    assert_eq!(initialize_response["serverInfo"]["name"], "mcp");

    // Verify capabilities structure
    assert!(initialize_response["capabilities"].is_object());
    assert!(initialize_response["capabilities"]["tools"].is_object());
}

/// Test response header management
#[test]
fn test_response_header_management() {
    let mut headers = HeaderMap::new();
    let session_id = Uuid::new_v4();

    // Test standard headers
    set_standard_headers(&mut headers, Some(session_id));

    assert_eq!(
        headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        headers.get(MCP_SESSION_ID).unwrap(),
        HeaderValue::from_str(&session_id.to_string()).unwrap()
    );

    // Test JSON response headers
    let mut json_headers = HeaderMap::new();
    set_json_response_headers(&mut json_headers, Some(session_id));

    assert_eq!(
        json_headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        json_headers.get(MCP_SESSION_ID).unwrap(),
        HeaderValue::from_str(&session_id.to_string()).unwrap()
    );
    assert_eq!(
        json_headers.get("content-type").unwrap(),
        HeaderValue::from_static(CONTENT_TYPE_JSON)
    );
}

/// Test protocol version error responses
#[test]
fn test_protocol_version_error_responses() {
    let unsupported_error = ProtocolVersionError::UnsupportedVersion(
        "2024-11-05".to_string(),
        "2025-06-18".to_string(),
    );

    let response = unsupported_error.into_response();

    // Verify error response status
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Verify response headers include protocol version
    let headers = response.headers();
    assert_eq!(
        headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        headers.get("content-type").unwrap(),
        HeaderValue::from_static(CONTENT_TYPE_JSON)
    );
}

/// Test header extractor with valid and invalid versions
#[tokio::test]
async fn test_header_extractor_integration() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    // Test with valid protocol version
    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(MCP_PROTOCOL_VERSION, "2025-06-18")
        .body(())
        .unwrap()
        .into_parts();

    let result = McpProtocolVersionHeader::from_request_parts(&mut parts, &()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().version, "2025-06-18");

    // Test with invalid protocol version
    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(MCP_PROTOCOL_VERSION, "2024-11-05")
        .body(())
        .unwrap()
        .into_parts();

    let result = McpProtocolVersionHeader::from_request_parts(&mut parts, &()).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(matches!(
        error,
        ProtocolVersionError::UnsupportedVersion(_, _)
    ));
}

/// Test complete end-to-end protocol negotiation flow
#[test]
fn test_end_to_end_protocol_negotiation() {
    // 1. Create session manager and session
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);
    let session_id = manager
        .create_session(Some(ClientInfo {
            user_agent: Some("test-client/1.0".to_string()),
            origin: Some("http://localhost:3001".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
        }))
        .unwrap();

    // 2. Validate session protocol version
    let session = manager.get_session(session_id).unwrap();
    assert_eq!(session.protocol_version, "2025-06-18");

    // 3. Validate protocol version via header validation
    let mut request_headers = HeaderMap::new();
    request_headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2025-06-18"));
    request_headers.insert(
        MCP_SESSION_ID,
        HeaderValue::from_str(&session_id.to_string()).unwrap(),
    );

    assert!(validate_protocol_version(&request_headers).is_ok());

    // 4. Create response headers
    let mut response_headers = HeaderMap::new();
    set_json_response_headers(&mut response_headers, Some(session_id));

    // 5. Verify all components use consistent protocol version
    assert_eq!(
        response_headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        response_headers.get(MCP_SESSION_ID).unwrap(),
        HeaderValue::from_str(&session_id.to_string()).unwrap()
    );
    assert_eq!(session.protocol_version, "2025-06-18");
}

/// Test protocol version constants consistency across modules
#[test]
fn test_protocol_version_constants_consistency() {
    // Verify all constants are consistent
    assert_eq!(SUPPORTED_PROTOCOL_VERSION, "2025-06-18");
    assert_eq!(PROTOCOL_SUPPORTED_VERSION, "2025-06-18");
    assert_eq!(ProtocolVersion::current().as_str(), "2025-06-18");

    // Verify registry uses consistent version
    let registry = ProtocolRegistry::new();
    assert_eq!(registry.current_version_string(), "2025-06-18");
    assert_eq!(registry.current_version().as_str(), "2025-06-18");
}

/// Test concurrent session creation maintains protocol version consistency
#[tokio::test]
async fn test_concurrent_protocol_version_consistency() {
    let config = SessionConfig::default();
    let manager = std::sync::Arc::new(SessionManager::new(config));

    let mut handles = Vec::new();

    // Create 50 concurrent sessions
    for i in 0..50 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let client_info = ClientInfo {
                user_agent: Some(format!("Client-{i}")),
                origin: Some("http://localhost:3001".to_string()),
                ip_address: Some("127.0.0.1".to_string()),
            };

            let session_id = manager_clone.create_session(Some(client_info)).unwrap();
            let session = manager_clone.get_session(session_id).unwrap();

            (session_id, session.protocol_version)
        });
        handles.push(handle);
    }

    let results: Vec<(Uuid, String)> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Verify all sessions have the same protocol version
    for (session_id, version) in &results {
        assert_eq!(version, "2025-06-18");

        // Double-check via manager validation
        assert!(manager
            .validate_session_protocol_version(*session_id, "2025-06-18")
            .is_ok());
    }

    // Verify all session IDs are unique
    let session_ids: std::collections::HashSet<_> = results.iter().map(|(id, _)| id).collect();
    assert_eq!(session_ids.len(), 50);
}

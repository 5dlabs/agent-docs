//! Comprehensive tests for MCP protocol version handling
//!
//! These tests verify the protocol version negotiation, validation, and error handling
//! functionality across the headers, session, and transport modules.

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
};
use chrono::Duration;
use doc_server_mcp::{
    headers::{
        extract_session_id, set_json_response_headers, set_standard_headers,
        validate_protocol_version, AcceptHeaderValidator, ContentTypeError, ContentTypeValidator,
        McpProtocolVersionHeader, ProtocolVersionError, CONTENT_TYPE_JSON, MCP_PROTOCOL_VERSION,
        MCP_SESSION_ID, SUPPORTED_PROTOCOL_VERSION,
    },
    protocol_version::SUPPORTED_PROTOCOL_VERSION as PROTOCOL_SUPPORTED_VERSION,
    session::{ClientInfo, Session, SessionConfig, SessionError, SessionManager},
};
use uuid::Uuid;

/// Test `SUPPORTED_PROTOCOL_VERSION` constants are consistent across modules
#[test]
fn test_protocol_version_constants_consistency() {
    assert_eq!(SUPPORTED_PROTOCOL_VERSION, "2025-06-18");
    assert_eq!(PROTOCOL_SUPPORTED_VERSION, "2025-06-18");
    assert_eq!(SUPPORTED_PROTOCOL_VERSION, PROTOCOL_SUPPORTED_VERSION);
}

/// Test basic protocol version validation with valid version
#[test]
fn test_validate_protocol_version_valid() {
    let mut headers = HeaderMap::new();
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2025-06-18"));

    let result = validate_protocol_version(&headers);
    assert!(result.is_ok());
}

/// Test protocol version validation with invalid version
#[test]
fn test_validate_protocol_version_invalid() {
    let mut headers = HeaderMap::new();
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2024-11-05"));

    let result = validate_protocol_version(&headers);
    assert_eq!(result, Err(StatusCode::BAD_REQUEST));
}

/// Test protocol version validation with missing header
#[test]
fn test_validate_protocol_version_missing() {
    let headers = HeaderMap::new();

    let result = validate_protocol_version(&headers);
    assert_eq!(result, Err(StatusCode::BAD_REQUEST));
}

/// Test protocol version validation with malformed header value
#[test]
fn test_validate_protocol_version_malformed() {
    let mut headers = HeaderMap::new();
    headers.insert(
        MCP_PROTOCOL_VERSION,
        HeaderValue::from_static("invalid-version"),
    );

    let result = validate_protocol_version(&headers);
    assert_eq!(result, Err(StatusCode::BAD_REQUEST));
}

/// Test `ProtocolVersionError` variants
#[test]
fn test_protocol_version_error_variants() {
    let missing_error = ProtocolVersionError::MissingHeader;
    let invalid_error = ProtocolVersionError::InvalidHeaderValue("test".to_string());
    let unsupported_error = ProtocolVersionError::UnsupportedVersion(
        "2024-11-05".to_string(),
        "2025-06-18".to_string(),
    );

    assert_eq!(
        missing_error.to_string(),
        "Missing MCP-Protocol-Version header"
    );
    assert_eq!(invalid_error.to_string(), "Invalid header value: test");
    assert_eq!(
        unsupported_error.to_string(),
        "Unsupported protocol version: 2024-11-05 (only 2025-06-18 supported)"
    );
}

/// Test `ProtocolVersionError` HTTP response conversion
#[test]
fn test_protocol_version_error_into_response() {
    let error = ProtocolVersionError::UnsupportedVersion(
        "2024-11-05".to_string(),
        "2025-06-18".to_string(),
    );

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Check that proper headers are set
    let headers = response.headers();
    assert!(headers.contains_key(MCP_PROTOCOL_VERSION));
    assert_eq!(
        headers.get("content-type").unwrap(),
        HeaderValue::from_static(CONTENT_TYPE_JSON)
    );
}

/// Test `McpProtocolVersionHeader` extractor with valid header
#[tokio::test]
async fn test_mcp_protocol_version_header_extractor_valid() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();
    parts
        .headers
        .insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2025-06-18"));

    let result = McpProtocolVersionHeader::from_request_parts(&mut parts, &()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().version, "2025-06-18");
}

/// Test `McpProtocolVersionHeader` extractor with invalid header
#[tokio::test]
async fn test_mcp_protocol_version_header_extractor_invalid() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();
    parts
        .headers
        .insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2024-11-05"));

    let result = McpProtocolVersionHeader::from_request_parts(&mut parts, &()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ProtocolVersionError::UnsupportedVersion(_, _)
    ));
}

/// Test `McpProtocolVersionHeader` extractor with missing header
#[tokio::test]
async fn test_mcp_protocol_version_header_extractor_missing() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();

    let result = McpProtocolVersionHeader::from_request_parts(&mut parts, &()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ProtocolVersionError::MissingHeader
    ));
}

/// Test `ContentTypeValidator` with valid JSON content type
#[tokio::test]
async fn test_content_type_validator_json() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();
    parts.headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json"),
    );

    let result = ContentTypeValidator::from_request_parts(&mut parts, &()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content_type, "application/json");
}

/// Test `ContentTypeValidator` with valid SSE content type
#[tokio::test]
async fn test_content_type_validator_sse() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();
    parts.headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("text/event-stream"),
    );

    let result = ContentTypeValidator::from_request_parts(&mut parts, &()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content_type, "text/event-stream");
}

/// Test `ContentTypeValidator` with invalid content type
#[tokio::test]
async fn test_content_type_validator_invalid() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();
    parts.headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("text/plain"),
    );

    let result = ContentTypeValidator::from_request_parts(&mut parts, &()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ContentTypeError::UnsupportedContentType(_)
    ));
}

/// Test `ContentTypeValidator` with missing content type
#[tokio::test]
async fn test_content_type_validator_missing() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()): (axum::http::request::Parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();

    let result = ContentTypeValidator::from_request_parts(&mut parts, &()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ContentTypeError::MissingHeader
    ));
}

/// Test session creation with default protocol version
#[test]
fn test_session_creation_with_default_protocol_version() {
    let ttl = Duration::minutes(30);
    let session = Session::new(ttl, None);

    assert_eq!(session.protocol_version, "2025-06-18");
    assert!(session.is_protocol_version_supported());
}

/// Test session creation with explicit protocol version
#[test]
fn test_session_creation_with_explicit_protocol_version() {
    let ttl = Duration::minutes(30);
    let session = Session::new_with_version(ttl, None, "2025-06-18".to_string());

    assert_eq!(session.protocol_version, "2025-06-18");
    assert!(session.is_protocol_version_supported());
}

/// Test session creation with unsupported protocol version
#[test]
fn test_session_creation_with_unsupported_protocol_version() {
    let ttl = Duration::minutes(30);
    let session = Session::new_with_version(ttl, None, "2024-11-05".to_string());

    assert_eq!(session.protocol_version, "2024-11-05");
    assert!(!session.is_protocol_version_supported());
}

/// Test session protocol version validation
#[test]
fn test_session_protocol_version_validation() {
    let ttl = Duration::minutes(30);
    let session = Session::new(ttl, None);

    // Valid version
    let result = session.validate_protocol_version("2025-06-18");
    assert!(result.is_ok());

    // Invalid version
    let result = session.validate_protocol_version("2024-11-05");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SessionError::ProtocolVersionMismatch { .. }
    ));
}

/// Test session manager creates sessions with correct protocol version
#[test]
fn test_session_manager_creates_sessions_with_protocol_version() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let session_id = manager.create_session(None).unwrap();
    let session = manager.get_session(session_id).unwrap();

    assert_eq!(session.protocol_version, "2025-06-18");
    assert!(session.is_protocol_version_supported());
}

/// Test session manager creates sessions with explicit protocol version
#[test]
fn test_session_manager_creates_sessions_with_explicit_version() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let session_id = manager
        .create_session_with_version(None, "2025-06-18")
        .unwrap();
    let session = manager.get_session(session_id).unwrap();

    assert_eq!(session.protocol_version, "2025-06-18");
    assert!(session.is_protocol_version_supported());
}

/// Test session manager protocol version validation
#[test]
fn test_session_manager_protocol_version_validation() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let session_id = manager.create_session(None).unwrap();

    // Valid version validation
    let result = manager.validate_session_protocol_version(session_id, "2025-06-18");
    assert!(result.is_ok());

    // Invalid version validation
    let result = manager.validate_session_protocol_version(session_id, "2024-11-05");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SessionError::ProtocolVersionMismatch { .. }
    ));
}

/// Test session manager validation with non-existent session
#[test]
fn test_session_manager_validation_nonexistent_session() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let random_uuid = Uuid::new_v4();
    let result = manager.validate_session_protocol_version(random_uuid, "2025-06-18");

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SessionError::SessionNotFound(_)
    ));
}

/// Test header setting functions
#[test]
fn test_set_standard_headers() {
    let mut headers = HeaderMap::new();
    let session_id = Uuid::new_v4();

    set_standard_headers(&mut headers, Some(session_id));

    assert_eq!(
        headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        headers.get(MCP_SESSION_ID).unwrap(),
        HeaderValue::from_str(&session_id.to_string()).unwrap()
    );
}

/// Test JSON response header setting
#[test]
fn test_set_json_response_headers() {
    let mut headers = HeaderMap::new();
    let session_id = Uuid::new_v4();

    set_json_response_headers(&mut headers, Some(session_id));

    assert_eq!(
        headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        headers.get(MCP_SESSION_ID).unwrap(),
        HeaderValue::from_str(&session_id.to_string()).unwrap()
    );
    assert_eq!(
        headers.get("content-type").unwrap(),
        HeaderValue::from_static(CONTENT_TYPE_JSON)
    );
}

/// Test session ID extraction from headers
#[test]
fn test_extract_session_id_valid() {
    let mut headers = HeaderMap::new();
    let session_id = Uuid::new_v4();
    headers.insert(
        MCP_SESSION_ID,
        HeaderValue::from_str(&session_id.to_string()).unwrap(),
    );

    let result = extract_session_id(&headers).unwrap();
    assert_eq!(result, Some(session_id));
}

/// Test session ID extraction with missing header
#[test]
fn test_extract_session_id_missing() {
    let headers = HeaderMap::new();

    let result = extract_session_id(&headers).unwrap();
    assert_eq!(result, None);
}

/// Test session ID extraction with invalid format
#[test]
fn test_extract_session_id_invalid() {
    let mut headers = HeaderMap::new();
    headers.insert(MCP_SESSION_ID, HeaderValue::from_static("not-a-uuid"));

    let result = extract_session_id(&headers);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid session ID format"));
}

/// Test protocol version mismatch error formatting
#[test]
fn test_protocol_version_mismatch_error_formatting() {
    let error = SessionError::ProtocolVersionMismatch {
        session_version: "2024-11-05".to_string(),
        expected_version: "2025-06-18".to_string(),
    };

    let error_string = error.to_string();
    assert!(error_string.contains("2024-11-05"));
    assert!(error_string.contains("2025-06-18"));
    assert!(error_string.contains("Protocol version mismatch"));
}

/// Test concurrent session creation with protocol version consistency
#[tokio::test]
async fn test_concurrent_session_protocol_version_consistency() {
    let config = SessionConfig::default();
    let manager = std::sync::Arc::new(SessionManager::new(config));

    let mut handles = Vec::new();

    for i in 0..20 {
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

    // All sessions should have the same protocol version
    for (_, version) in &results {
        assert_eq!(version, "2025-06-18");
    }

    // All session IDs should be unique
    let session_ids: std::collections::HashSet<_> = results.iter().map(|(id, _)| id).collect();
    assert_eq!(session_ids.len(), 20);
}

/// Test session with different client info maintains protocol version
#[test]
fn test_session_client_info_protocol_version_consistency() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let client_infos = vec![
        Some(ClientInfo {
            user_agent: Some("Chrome/120.0".to_string()),
            origin: Some("http://localhost:3001".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
        }),
        Some(ClientInfo {
            user_agent: Some("Firefox/121.0".to_string()),
            origin: Some("https://localhost:3001".to_string()),
            ip_address: Some("::1".to_string()),
        }),
        None,
    ];

    for client_info in client_infos {
        let session_id = manager.create_session(client_info).unwrap();
        let session = manager.get_session(session_id).unwrap();

        assert_eq!(session.protocol_version, "2025-06-18");
        assert!(session.is_protocol_version_supported());
    }
}

/// Test header extraction with various edge cases
#[test]
fn test_header_edge_cases() {
    // Empty protocol version
    let mut headers = HeaderMap::new();
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static(""));
    assert_eq!(
        validate_protocol_version(&headers),
        Err(StatusCode::BAD_REQUEST)
    );

    // Protocol version with extra whitespace should pass (trim allowed)
    let mut headers = HeaderMap::new();
    headers.insert(
        MCP_PROTOCOL_VERSION,
        HeaderValue::from_static(" 2025-06-18 "),
    );
    assert!(validate_protocol_version(&headers).is_ok());

    // Case sensitivity test
    let mut headers = HeaderMap::new();
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2025-06-18"));
    assert!(validate_protocol_version(&headers).is_ok());

    let mut headers = HeaderMap::new();
    headers.insert(MCP_PROTOCOL_VERSION, HeaderValue::from_static("2025-06-18"));
    assert!(validate_protocol_version(&headers).is_ok());
}

/// Test Accept header validation with valid cases
#[test]
fn test_accept_header_validator_valid_cases() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    // Test with application/json
    let (mut parts, ()) = Request::builder()
        .method("POST")
        .uri("/")
        .header("accept", "application/json")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_ok());

    // Test with application/*
    let (mut parts, ()) = Request::builder()
        .method("POST")
        .uri("/")
        .header("accept", "application/*")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_ok());

    // Test with */*
    let (mut parts, ()) = Request::builder()
        .method("POST")
        .uri("/")
        .header("accept", "*/*")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_ok());

    // Test with text/event-stream (for future SSE support)
    let (mut parts, ()) = Request::builder()
        .method("GET")
        .uri("/")
        .header("accept", "text/event-stream")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_ok());
}

/// Test Accept header validation with invalid cases
#[test]
fn test_accept_header_validator_invalid_cases() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;
    use doc_server_mcp::headers::AcceptHeaderError;

    // Test with unacceptable media type
    let (mut parts, ()) = Request::builder()
        .method("POST")
        .uri("/")
        .header("accept", "text/plain")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AcceptHeaderError::UnacceptableMediaType(_)
    ));

    // Test with xml (also unacceptable)
    let (mut parts, ()) = Request::builder()
        .method("POST")
        .uri("/")
        .header("accept", "application/xml")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AcceptHeaderError::UnacceptableMediaType(_)
    ));
}

/// Test Accept header validation with missing header (should be OK)
#[test]
fn test_accept_header_validator_missing_header() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let (mut parts, ()) = Request::builder()
        .method("POST")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts();

    let result = tokio_test::block_on(AcceptHeaderValidator::from_request_parts(&mut parts, &()));
    assert!(result.is_ok());
}

/// Integration test: Full protocol version validation workflow
#[test]
fn test_protocol_version_integration_workflow() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    // 1. Create session with default protocol version
    let session_id = manager.create_session(None).unwrap();

    // 2. Validate the session has correct protocol version
    let session = manager.get_session(session_id).unwrap();
    assert_eq!(session.protocol_version, "2025-06-18");
    assert!(session.is_protocol_version_supported());

    // 3. Validate protocol version consistency
    let validation_result = manager.validate_session_protocol_version(session_id, "2025-06-18");
    assert!(validation_result.is_ok());

    // 4. Test rejection of wrong version
    let wrong_validation = manager.validate_session_protocol_version(session_id, "2024-11-05");
    assert!(wrong_validation.is_err());

    // 5. Create response headers and verify protocol version is included
    let mut response_headers = HeaderMap::new();
    set_json_response_headers(&mut response_headers, Some(session_id));

    assert_eq!(
        response_headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        HeaderValue::from_static("2025-06-18")
    );
    assert_eq!(
        response_headers.get(MCP_SESSION_ID).unwrap(),
        HeaderValue::from_str(&session_id.to_string()).unwrap()
    );
    assert_eq!(
        response_headers.get("content-type").unwrap(),
        HeaderValue::from_static("application/json")
    );
}

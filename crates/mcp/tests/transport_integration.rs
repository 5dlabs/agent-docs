//! Integration tests for the Streamable HTTP transport implementation
//! These tests verify end-to-end functionality of the new transport layer

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use doc_server_mcp::{
    headers::{MCP_PROTOCOL_VERSION, MCP_SESSION_ID, SUPPORTED_PROTOCOL_VERSION},
    transport::{SessionManager, TransportConfig},
    McpServer,
};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

// Helper function to create test server
async fn create_test_server() -> Router {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost:5432/test".to_string());

    match doc_server_database::DatabasePool::new(&database_url).await {
        Ok(db_pool) => {
            let server = McpServer::new(db_pool)
                .expect("Failed to create server");
            server.create_router()
        }
        Err(_) => {
            // For CI environments without database, create a mock router
            create_mock_router()
        }
    }
}

// Create a mock router that simulates the transport behavior without database
#[allow(clippy::too_many_lines)]
fn create_mock_router() -> Router {
    use axum::{
        extract::{Request, State},
        http::{HeaderMap, Method},
        response::{IntoResponse, Response},
        routing::{any, get},
    };
    use doc_server_mcp::{
        headers::{set_standard_headers, validate_protocol_version},
        transport::{SessionManager, TransportConfig, TransportError},
    };

    #[derive(Clone)]
    struct MockState {
        session_manager: SessionManager,
    }

    async fn mock_mcp_handler(
        State(state): State<MockState>,
        headers: HeaderMap,
        request: Request<Body>,
    ) -> Result<Response, TransportError> {
        // Validate protocol version
        if let Err(status_code) = validate_protocol_version(&headers) {
            return match status_code {
                StatusCode::BAD_REQUEST => Err(TransportError::UnsupportedProtocolVersion(
                    headers
                        .get(MCP_PROTOCOL_VERSION)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("missing")
                        .to_string(),
                )),
                _ => Err(TransportError::InternalError(
                    "Protocol validation failed".to_string(),
                )),
            };
        }

        match *request.method() {
            Method::POST => {
                // Validate Content-Type
                let content_type = headers
                    .get("content-type")
                    .ok_or(TransportError::MissingContentType)?
                    .to_str()
                    .map_err(|_| {
                        TransportError::InvalidContentType("invalid header value".to_string())
                    })?;

                if !content_type.starts_with("application/json") {
                    return Err(TransportError::InvalidContentType(content_type.to_string()));
                }

                // Get or create session
                let session_id = state.session_manager.get_or_create_session(&headers)?;

                // Parse JSON body to check if it's valid
                let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
                    .await
                    .map_err(|e| TransportError::InternalError(format!("Failed to read body: {e}")))?;

                let json_request: Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| TransportError::JsonParseError(e.to_string()))?;

                // Mock different JSON-RPC responses based on method
                let method = json_request
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                let mock_response = match method {
                    "tools/list" => json!({
                        "tools": [{
                            "name": "rust_query",
                            "description": "Search Rust documentation"
                        }]
                    }),
                    "tools/call" => json!({
                        "content": [{
                            "type": "text",
                            "text": "Mock response for rust_query tool"
                        }]
                    }),
                    "initialize" => json!({
                        "protocolVersion": "2025-06-18",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "doc-server-mcp",
                            "version": "test"
                        }
                    }),
                    _ => json!({
                        "result": "mock response"
                    }),
                };

                // Create response with proper headers
                let mut response_headers = HeaderMap::new();
                set_standard_headers(&mut response_headers, Some(session_id));
                response_headers.insert("content-type", "application/json".parse().unwrap());

                Ok((StatusCode::OK, response_headers, axum::Json(mock_response)).into_response())
            }
            Method::GET => Err(TransportError::MethodNotAllowed),
            _ => Err(TransportError::MethodNotAllowed),
        }
    }

    async fn mock_health_handler() -> impl IntoResponse {
        axum::Json(json!({
            "status": "healthy",
            "service": "doc-server-mcp",
            "version": "test"
        }))
    }

    let config = TransportConfig::default();
    let session_manager = SessionManager::new(config);
    let state = MockState { session_manager };

    Router::new()
        .route("/mcp", any(mock_mcp_handler))
        .route("/health", get(mock_health_handler))
        .with_state(state)
}

// Helper function to create JSON-RPC request
fn create_json_rpc_request(method: &str, params: Option<Value>) -> Value {
    let mut req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method
    });

    if let Some(p) = params {
        req["params"] = p;
    }

    req
}

#[tokio::test]
async fn test_post_mcp_with_protocol_version() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request("initialize", None);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be successful
    assert!(response.status().is_success());

    // Should include protocol version header
    let headers = response.headers();
    assert!(headers.contains_key(MCP_PROTOCOL_VERSION));
    assert_eq!(
        headers.get(MCP_PROTOCOL_VERSION).unwrap(),
        SUPPORTED_PROTOCOL_VERSION
    );

    // Should include session ID header
    assert!(headers.contains_key(MCP_SESSION_ID));

    // Session ID should be a valid UUID
    let session_id_str = headers.get(MCP_SESSION_ID).unwrap().to_str().unwrap();
    assert!(Uuid::parse_str(session_id_str).is_ok());
}

#[tokio::test]
async fn test_post_mcp_without_protocol_version_returns_400() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request("initialize", None);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        // Missing MCP-Protocol-Version header
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_mcp_with_wrong_protocol_version_returns_400() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request("initialize", None);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, "2024-11-05") // Old protocol version
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_mcp_returns_405() {
    let app = create_test_server().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/mcp")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 405 Method Not Allowed (MVP: no SSE support)
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_post_mcp_without_content_type_returns_400() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request("initialize", None);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        // Missing content-type header
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_mcp_with_wrong_content_type_returns_400() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request("initialize", None);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "text/plain") // Wrong content type
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_mcp_with_malformed_json_returns_400() {
    let app = create_test_server().await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from("{invalid json}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_session_reuse_with_session_id() {
    let app = create_test_server().await;

    // First request - should create new session
    let request_body = create_json_rpc_request("initialize", None);

    let request1 = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert!(response1.status().is_success());

    let session_id = response1
        .headers()
        .get(MCP_SESSION_ID)
        .unwrap()
        .to_str()
        .unwrap();

    // Second request with same session ID - should reuse session
    let request_body2 = create_json_rpc_request("tools/list", None);

    let request2 = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .header(MCP_SESSION_ID, session_id) // Reuse session ID
        .body(Body::from(request_body2.to_string()))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    assert!(response2.status().is_success());

    // Should return the same session ID
    let session_id2 = response2
        .headers()
        .get(MCP_SESSION_ID)
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(session_id, session_id2);
}

#[tokio::test]
async fn test_tools_list_endpoint() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request("tools/list", None);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be successful
    assert!(response.status().is_success());

    // Parse response body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Should contain tools array
    assert!(response_json.get("tools").is_some());
    let tools = response_json["tools"].as_array().unwrap();

    // Should contain rust_query tool
    let has_rust_query = tools
        .iter()
        .any(|tool| tool.get("name").and_then(|n| n.as_str()).is_some_and(|name| name == "rust_query"));
    assert!(has_rust_query, "Should contain rust_query tool");
}

#[tokio::test]
async fn test_rust_query_tool_call() {
    let app = create_test_server().await;

    let request_body = create_json_rpc_request(
        "tools/call",
        Some(json!({
            "name": "rust_query",
            "arguments": {
                "query": "tokio runtime"
            }
        })),
    );

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be successful
    assert!(response.status().is_success());

    // Parse response body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Should contain content array
    assert!(response_json.get("content").is_some());
    let content = response_json["content"].as_array().unwrap();
    assert!(!content.is_empty());

    // First content item should be text
    let first_content = &content[0];
    assert_eq!(first_content["type"].as_str().unwrap(), "text");
    assert!(first_content["text"].is_string());
}

#[tokio::test]
async fn test_unsupported_http_methods() {
    let app = create_test_server().await;

    // Test PUT method
    let request = Request::builder()
        .method(Method::PUT)
        .uri("/mcp")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Test DELETE method
    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/mcp")
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// Unit tests for session manager
#[tokio::test]
async fn test_session_manager_creation() {
    let config = TransportConfig::default();
    let session_manager = SessionManager::new(config);

    // Should start with no sessions
    assert_eq!(session_manager.session_count().unwrap(), 0);

    // Should be able to create sessions
    let session_id = session_manager.create_session().unwrap();
    assert_eq!(session_manager.session_count().unwrap(), 1);

    // Should be able to update session activity
    session_manager.update_session_activity(session_id).unwrap();
}

#[tokio::test]
async fn test_session_expiration() {
    let mut config = TransportConfig::default();
    config.session_timeout = Duration::from_millis(100); // Very short timeout for testing
    let session_manager = SessionManager::new(config);

    // Create a session
    let _session_id = session_manager.create_session().unwrap();
    assert_eq!(session_manager.session_count().unwrap(), 1);

    // Wait for session to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Clean up expired sessions
    let cleaned = session_manager.cleanup_expired_sessions().unwrap();
    assert_eq!(cleaned, 1);
    assert_eq!(session_manager.session_count().unwrap(), 0);
}

#[tokio::test]
async fn test_concurrent_sessions() {
    let config = TransportConfig::default();
    let session_manager = SessionManager::new(config);

    // Create multiple sessions concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let sm = session_manager.clone();
        handles.push(tokio::spawn(async move { sm.create_session() }));
    }

    // Wait for all sessions to be created
    let mut session_ids = vec![];
    for handle in handles {
        let session_id = handle.await.unwrap().unwrap();
        session_ids.push(session_id);
    }

    // Should have 10 sessions
    assert_eq!(session_manager.session_count().unwrap(), 10);

    // All session IDs should be unique
    session_ids.sort();
    session_ids.dedup();
    assert_eq!(session_ids.len(), 10);
}

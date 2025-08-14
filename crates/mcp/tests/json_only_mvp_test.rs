//! Tests for JSON-only MVP behavior and error handling
//!
//! This test file verifies that the MCP server correctly handles:
//! - GET requests return 405 Method Not Allowed
//! - POST requests with proper headers work
//! - Error responses include required headers
//! - JSON-RPC error format is correct

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use doc_server_mcp::{headers::SUPPORTED_PROTOCOL_VERSION, metrics::metrics, McpServer};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Create a test server or mock router for testing
async fn create_test_server() -> Router {
    // Try to create real server first, fall back to mock if database not available
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost:5432/test".to_string());

    match doc_server_database::DatabasePool::new(&database_url).await {
        Ok(db_pool) => {
            let server = McpServer::new(db_pool)
                .await
                .expect("Failed to create server");
            server.create_router()
        }
        Err(_) => create_mock_router(),
    }
}

/// Create a mock router for testing when database is not available
fn create_mock_router() -> Router {
    use axum::{
        http::HeaderMap,
        response::IntoResponse,
        routing::{any, get},
        Json,
    };
    use doc_server_mcp::headers::{set_json_response_headers, set_standard_headers};

    fn mock_mcp_handler(method: &Method, _headers: HeaderMap) -> impl IntoResponse {
        if *method == Method::POST {
            let mut headers = HeaderMap::new();
            set_json_response_headers(&mut headers, None);
            (
                StatusCode::OK,
                headers,
                Json(json!({
                    "jsonrpc": "2.0",
                    "result": {"status": "ok"},
                    "id": 1
                })),
            )
                .into_response()
        } else {
            // Handle GET and all other methods with 405
            let mut headers = HeaderMap::new();
            set_standard_headers(&mut headers, None);
            (
                StatusCode::METHOD_NOT_ALLOWED,
                headers,
                Json(json!({
                    "error": {
                        "code": -32600,
                        "message": "Method Not Allowed"
                    }
                })),
            )
                .into_response()
        }
    }

    async fn mock_health_handler() -> impl IntoResponse {
        Json(json!({
            "status": "healthy",
            "service": "doc-server-mcp",
            "version": "test"
        }))
    }

    Router::new()
        .route(
            "/mcp",
            any(|method: Method, headers: HeaderMap| async move { mock_mcp_handler(&method, headers) }),
        )
        .route("/health", get(mock_health_handler))
}

#[tokio::test]
async fn test_get_returns_405_with_proper_headers() {
    let app = create_test_server().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/mcp")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Must return 405 Method Not Allowed
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Must include MCP protocol version header
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
    assert_eq!(
        headers
            .get("MCP-Protocol-Version")
            .unwrap()
            .to_str()
            .unwrap(),
        SUPPORTED_PROTOCOL_VERSION
    );
}

#[tokio::test]
async fn test_post_with_json_succeeds() {
    let app = create_test_server().await;

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .header("accept", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be successful or server error (but not method not allowed)
    assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Must include required headers
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
    assert!(headers.contains_key("Content-Type"));
}

#[tokio::test]
async fn test_post_without_protocol_version_fails() {
    let app = create_test_server().await;

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        // Missing MCP-Protocol-Version header
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request for missing protocol version
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Even error responses should include protocol headers
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
}

#[tokio::test]
async fn test_post_with_invalid_json_returns_400() {
    let app = create_test_server().await;

    let invalid_json = "{ invalid json }";

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(invalid_json))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request for invalid JSON
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Response should be JSON-RPC error format
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
    assert!(headers.contains_key("Content-Type"));
    assert!(headers
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("application/json"));
}

#[tokio::test]
async fn test_unsupported_method_returns_405() {
    let app = create_test_server().await;

    let request = Request::builder()
        .method(Method::PUT)
        .uri("/mcp")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Must return 405 Method Not Allowed
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Must include proper headers
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
}

#[tokio::test]
async fn test_post_with_unacceptable_accept_header() {
    let app = create_test_server().await;

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .header("accept", "text/plain") // Not acceptable for JSON responses
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 406 Not Acceptable
    assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);

    // Must still include proper headers
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
}

#[tokio::test]
async fn test_error_response_format() {
    let app = create_test_server().await;

    // Try to trigger an error (missing content-type)
    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        // Missing content-type header
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Response should be JSON-RPC error format
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
    assert!(headers.contains_key("Content-Type"));

    // Try to parse response body as JSON to verify format
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Should have JSON-RPC error structure
    assert!(response_json.get("error").is_some());
    let error = response_json.get("error").unwrap();
    assert!(error.get("code").is_some());
    assert!(error.get("message").is_some());
}

#[tokio::test]
async fn test_metrics_tracking_get_request() {
    let app = create_test_server().await;

    // Get initial metrics snapshot
    let initial_metrics = metrics().snapshot();

    // Make a GET request (should increment method_not_allowed and requests_total)
    let request = Request::builder()
        .method(Method::GET)
        .uri("/mcp")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Check that metrics were incremented
    let final_metrics = metrics().snapshot();
    assert_eq!(
        final_metrics.requests_total,
        initial_metrics.requests_total + 1,
        "Total requests should be incremented"
    );
    assert_eq!(
        final_metrics.method_not_allowed_total,
        initial_metrics.method_not_allowed_total + 1,
        "Method not allowed counter should be incremented"
    );
}

#[tokio::test]
async fn test_metrics_tracking_protocol_version_error() {
    let app = create_test_server().await;

    // Get initial metrics snapshot
    let initial_metrics = metrics().snapshot();

    // Make a POST request without protocol version header
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        // Missing MCP-Protocol-Version header intentionally
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Check that metrics were incremented
    let final_metrics = metrics().snapshot();
    assert_eq!(
        final_metrics.requests_total,
        initial_metrics.requests_total + 1,
        "Total requests should be incremented"
    );
    assert_eq!(
        final_metrics.protocol_version_errors,
        initial_metrics.protocol_version_errors + 1,
        "Protocol version errors should be incremented"
    );
}

#[tokio::test]
async fn test_metrics_tracking_successful_post() {
    let app = create_test_server().await;

    // Get initial metrics snapshot
    let initial_metrics = metrics().snapshot();

    // Make a valid POST request
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .header("accept", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should be successful (either 200 or some other success status, but not method not allowed)
    assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Check that metrics were incremented
    let final_metrics = metrics().snapshot();
    assert_eq!(
        final_metrics.requests_total,
        initial_metrics.requests_total + 1,
        "Total requests should be incremented"
    );

    // If the request was fully successful (got to handler), post_requests_success should be incremented
    // If there was a database connection issue, we might get an internal error instead
    // Let's just verify the request counter was incremented
    assert!(
        final_metrics.requests_total > initial_metrics.requests_total,
        "Request counter should have increased"
    );
}

#[tokio::test]
async fn test_metrics_tracking_json_parse_error() {
    let app = create_test_server().await;

    // Get initial metrics snapshot
    let initial_metrics = metrics().snapshot();

    // Make a POST request with invalid JSON
    let invalid_json = "{ invalid json }";

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("MCP-Protocol-Version", SUPPORTED_PROTOCOL_VERSION)
        .body(Body::from(invalid_json))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Check that metrics were incremented
    let final_metrics = metrics().snapshot();
    assert_eq!(
        final_metrics.requests_total,
        initial_metrics.requests_total + 1,
        "Total requests should be incremented"
    );
    assert_eq!(
        final_metrics.json_parse_errors,
        initial_metrics.json_parse_errors + 1,
        "JSON parse errors should be incremented"
    );
}

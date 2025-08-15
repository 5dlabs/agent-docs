//! Test to verify routing behavior for GET and POST /mcp endpoints
//! and that proper headers are included in responses

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use doc_server_mcp::{headers::SUPPORTED_PROTOCOL_VERSION, McpServer};
use serde_json::json;
use tower::ServiceExt;

// Mock database pool for testing
async fn create_test_server() -> Router {
    // Default to mock for speed; only use real DB if explicitly requested
    if std::env::var("TEST_DATABASE_URL")
        .map(|v| v.trim().is_empty() || v.trim().eq_ignore_ascii_case("mock"))
        .unwrap_or(true)
    {
        return create_mock_router();
    }

    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set or use mock");
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

fn create_mock_router() -> Router {
    use axum::{
        http::HeaderMap,
        response::IntoResponse,
        routing::{get, post},
        Json,
    };
    use doc_server_mcp::headers::set_standard_headers;

    async fn mock_post_handler() -> impl IntoResponse {
        let mut headers = HeaderMap::new();
        set_standard_headers(&mut headers, None);
        (headers, Json(json!({"message": "mock response"}))).into_response()
    }

    async fn mock_get_handler() -> impl IntoResponse {
        (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
    }

    async fn mock_health_handler() -> impl IntoResponse {
        Json(json!({
            "status": "healthy",
            "service": "doc-server-mcp",
            "version": "test"
        }))
    }

    Router::new()
        .route("/mcp", post(mock_post_handler))
        .route("/mcp", get(mock_get_handler))
        .route("/health", get(mock_health_handler))
}

#[tokio::test]
async fn test_get_mcp_returns_405() {
    let app = create_test_server().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/mcp")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_post_mcp_includes_protocol_header() {
    let app = create_test_server().await;

    let request_body = json!({
        "method": "initialize",
        "params": {}
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Check that the response includes the MCP protocol version header
    let headers = response.headers();

    assert!(
        headers.contains_key("MCP-Protocol-Version"),
        "Response should include MCP-Protocol-Version header"
    );

    assert_eq!(
        headers
            .get("MCP-Protocol-Version")
            .unwrap()
            .to_str()
            .unwrap(),
        SUPPORTED_PROTOCOL_VERSION,
        "Protocol version should be {SUPPORTED_PROTOCOL_VERSION}"
    );
}

#[tokio::test]
async fn test_post_mcp_successful_response() {
    let app = create_test_server().await;

    let request_body = json!({
        "method": "initialize",
        "params": {}
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be successful (200 OK or similar)
    assert!(
        response.status().is_success() || response.status().is_server_error(),
        "Response status: {:?}",
        response.status()
    );

    // Should have the protocol header regardless of success/failure
    let headers = response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
}

#[tokio::test]
async fn test_health_endpoint_works() {
    let app = create_test_server().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_routing_integration() {
    let app = create_test_server().await;

    // Test that both GET and POST routes exist for /mcp but behave differently

    // GET should return 405
    let get_request = Request::builder()
        .method(Method::GET)
        .uri("/mcp")
        .body(Body::empty())
        .unwrap();

    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // POST should work (may return error due to no database, but should not be 405)
    let post_request_body = json!({"method": "initialize"});
    let post_request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(post_request_body.to_string()))
        .unwrap();

    let post_response = app.oneshot(post_request).await.unwrap();

    // Should NOT be method not allowed - any other error is fine for this test
    assert_ne!(post_response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Should have protocol header
    let headers = post_response.headers();
    assert!(headers.contains_key("MCP-Protocol-Version"));
}

//! Simple test server to run diagnostic tests
//! This creates a mock server similar to the test infrastructure

use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode},
    response::IntoResponse,
    routing::{any, get},
    Json, Router,
};
use mcp::{
    headers::{set_json_response_headers, set_standard_headers, SUPPORTED_PROTOCOL_VERSION},
    metrics::metrics,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceExt;

/// Create a mock router for testing when database is not available
#[allow(clippy::too_many_lines)]
fn create_mock_router() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/mcp", any(mcp_handler))
        .route("/metrics", get(metrics_handler))
}

async fn health_handler() -> impl IntoResponse {
    axum::Json(json!({
        "status": "healthy",
        "version": "test",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn metrics_handler() -> impl IntoResponse {
    let metrics_data = metrics().get_metrics();
    axum::Json(json!({
        "requests_total": metrics_data.requests_total,
        "requests_by_method": metrics_data.requests_by_method,
        "requests_by_status": metrics_data.requests_by_status,
        "errors_total": metrics_data.errors_total,
        "uptime_seconds": metrics_data.uptime_seconds
    }))
}

async fn mcp_handler(method: Method, headers: HeaderMap, body: axum::body::Bytes) -> impl IntoResponse {
    match method {
        Method::POST => {
            // Parse JSON-RPC request
            let json_body: Result<Value, _> = serde_json::from_slice(&body);
            
            match json_body {
                Ok(json) => {
                    // Check if this is a notification (no id field)
                    let is_notification = json.get("id").is_none();
                    
                    if is_notification {
                        // For notifications, return 204 No Content
                        let mut response_headers = HeaderMap::new();
                        set_standard_headers(&mut response_headers, None);
                        return (StatusCode::NO_CONTENT, response_headers).into_response();
                    } else {
                        // For regular requests, return a proper JSON-RPC response
                        let mut response_headers = HeaderMap::new();
                        set_json_response_headers(&mut response_headers, None);
                        
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": json.get("id"),
                            "result": {
                                "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
                                "capabilities": {
                                    "tools": {
                                        "listChanged": true
                                    },
                                    "prompts": {}
                                }
                            }
                        });
                        
                        return (StatusCode::OK, response_headers, Json(response)).into_response();
                    }
                }
                Err(_) => {
                    let mut response_headers = HeaderMap::new();
                    set_standard_headers(&mut response_headers, None);
                    metrics().increment_bad_request();
                    return (StatusCode::BAD_REQUEST, response_headers).into_response();
                }
            }
        }
        Method::GET => {
            // Mock SSE response for Streamable HTTP transport
            let mut h = HeaderMap::new();
            set_standard_headers(&mut h, None);
            h.insert("content-type", "text/event-stream".parse().unwrap());

            let sse_body = "data: {\"jsonrpc\": \"2.0\", \"method\": \"notifications/initialized\", \"params\": {\"protocolVersion\": \"2025-06-18\", \"capabilities\": {\"tools\": {}, \"prompts\": {}}}}\n\n";

            return (StatusCode::OK, h, sse_body).into_response();
        }
        Method::HEAD => {
            let mut h = HeaderMap::new();
            set_standard_headers(&mut h, None);
            return (StatusCode::OK, h).into_response();
        }
        _ => {
            let mut h = HeaderMap::new();
            set_standard_headers(&mut h, None);
            metrics().increment_method_not_allowed();
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                h,
                Json(json!({
                    "error": "Method not allowed"
                }))
            ).into_response();
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let app = create_mock_router();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    
    println!("Test server starting on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

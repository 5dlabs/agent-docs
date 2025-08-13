//! MCP transport layer
//!
//! This module provides the transport configuration, error types, and unified handler
//! for MCP requests over HTTP POST. Task 3 will implement the full transport logic.

use axum::extract::State;
use axum::http::{HeaderMap, Request};
use axum::response::{IntoResponse, Response};
use axum::body::Body;
use std::sync::Arc;

/// Transport configuration for MCP server
#[derive(Clone, Debug)]
pub struct TransportConfig {
    /// The MCP protocol version to use
    pub protocol_version: String,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            protocol_version: "2025-06-18".to_string(),
        }
    }
}

/// Transport layer errors
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Method not allowed - only POST is supported
    #[error("method not allowed")]
    MethodNotAllowed,
    /// Protocol version mismatch or validation error
    #[error("protocol version error: {0}")]
    ProtocolVersionError(String),
    /// Invalid request format
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// Internal server error
    #[error("internal error: {0}")]
    InternalError(String),
}

impl axum::response::IntoResponse for TransportError {
    fn into_response(self) -> Response<Body> {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, message) = match &self {
            TransportError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, self.to_string()),
            TransportError::ProtocolVersionError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            TransportError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            TransportError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

// Placeholder state type - Task 3 will define the actual state
pub struct McpServerState {
    pub config: TransportConfig,
}

/// Unified MCP handler for HTTP POST requests
/// 
/// This is a stub implementation that Task 3 will replace with full logic.
/// For now, it only returns MethodNotAllowed for non-POST requests.
pub async fn unified_mcp_handler(
    State(_state): State<Arc<McpServerState>>,
    _headers: HeaderMap,
    req: Request<Body>,
) -> Result<Response<Body>, TransportError> {
    // For MVP, we only check the HTTP method
    match req.method() {
        &axum::http::Method::POST => {
            // Task 3 will implement the actual request processing logic here
            // For now, return a placeholder success response
            use axum::Json;
            use serde_json::json;
            
            let response = Json(json!({
                "message": "MCP handler stub - Task 3 will implement full logic"
            }));
            
            Ok(response.into_response())
        }
        _ => Err(TransportError::MethodNotAllowed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.protocol_version, "2025-06-18");
    }

    #[test]
    fn test_transport_error_display() {
        let error = TransportError::MethodNotAllowed;
        assert_eq!(error.to_string(), "method not allowed");

        let error = TransportError::ProtocolVersionError("unsupported".to_string());
        assert_eq!(error.to_string(), "protocol version error: unsupported");
    }

    #[tokio::test]
    async fn test_unified_mcp_handler_post() {
        let config = TransportConfig::default();
        let state = Arc::new(McpServerState { config });
        let headers = HeaderMap::new();
        let req = Request::builder()
            .method(Method::POST)
            .body(Body::empty())
            .unwrap();

        let result = unified_mcp_handler(State(state), headers, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unified_mcp_handler_get_not_allowed() {
        let config = TransportConfig::default();
        let state = Arc::new(McpServerState { config });
        let headers = HeaderMap::new();
        let req = Request::builder()
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let result = unified_mcp_handler(State(state), headers, req).await;
        assert!(matches!(result, Err(TransportError::MethodNotAllowed)));
    }
}

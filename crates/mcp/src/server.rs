//! MCP server implementation

use crate::handlers::McpHandler;
use crate::headers::set_standard_headers;
use anyhow::Result;
use doc_server_database::DatabasePool;
// use crate::sse::sse_handler; // TODO: implement SSE handler
// Headers utilities are used in subsequent tasks; unused for MVP POST handler
// use crate::headers::{MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION};
use axum::{
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info};

/// MCP server state
#[derive(Clone)]
pub struct McpServerState {
    pub db_pool: DatabasePool,
    pub handler: Arc<McpHandler>,
}

/// MCP server
pub struct McpServer {
    state: McpServerState,
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        let handler = Arc::new(McpHandler::new(db_pool.clone()).await?);
        let state = McpServerState { db_pool, handler };

        Ok(Self { state })
    }

    /// Start serving on the given address
    pub async fn serve(&self, addr: &str) -> Result<()> {
        let app = self.create_router();

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("MCP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create the router with all endpoints
    pub fn create_router(&self) -> Router {
        Router::new()
            // Health check endpoint
            .route("/health", get(health_check))
            // TODO: MCP SSE endpoint for real-time communication
            // .route("/sse", get(sse_handler))
            // MCP JSON-RPC endpoint for tool calls
            .route("/mcp", post(mcp_handler).get(mcp_get_handler))
            // GET /mcp returns 405 Method Not Allowed
            // Add CORS for Toolman compatibility
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers(Any),
            )
            .with_state(self.state.clone())
    }
}

/// Health check endpoint
async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "doc-server-mcp",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// GET handler for /mcp - returns 405 Method Not Allowed  
async fn mcp_get_handler() -> impl IntoResponse {
    (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
}

/// MCP JSON-RPC handler for tool calls
async fn mcp_handler(
    State(state): State<McpServerState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    debug!("Received MCP request: {}", payload);

    // Create headers with the required MCP protocol version
    let mut headers = HeaderMap::new();
    set_standard_headers(&mut headers, None);

    match state.handler.handle_request(payload).await {
        Ok(response) => {
            debug!("MCP response: {}", response);
            (headers, Json(response)).into_response()
        }
        Err(e) => {
            error!("MCP request failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, headers, "Internal Server Error").into_response()
        }
    }
}

// legacy helper name removed; using `mcp_get_handler` above

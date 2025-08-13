//! MCP server implementation

use crate::handlers::McpHandler;
use anyhow::Result;
use doc_server_database::DatabasePool;
// use crate::sse::sse_handler; // TODO: implement SSE handler
use crate::headers::{MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION};
use axum::{
    extract::State,
    http::{HeaderMap, Method, StatusCode},
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
    fn create_router(&self) -> Router {
        Router::new()
            // Health check endpoint
            .route("/health", get(health_check))
            // TODO: MCP SSE endpoint for real-time communication
            // .route("/sse", get(sse_handler))
            // MCP JSON-RPC endpoint for tool calls
            .route("/mcp", post(mcp_handler).get(mcp_get_method_not_allowed))
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

/// MCP JSON-RPC handler for tool calls
async fn mcp_handler(
    State(state): State<McpServerState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Received MCP request: {}", payload);

    match state.handler.handle_request(payload).await {
        Ok(response) => {
            debug!("MCP response: {}", response);
            // Inject MCP-Protocol-Version header via axum response conversion by wrapping in Json
            // Note: For MVP, we ensure the header is present at router/middleware level in Task 3.
            Ok(Json(response))
        }
        Err(e) => {
            error!("MCP request failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET on /mcp should return 405 (for MVP)
async fn mcp_get_method_not_allowed(_headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    Err(StatusCode::METHOD_NOT_ALLOWED)
}

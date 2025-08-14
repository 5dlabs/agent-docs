//! MCP server implementation

use crate::handlers::McpHandler;
use crate::transport::{
    initialize_transport, unified_mcp_handler, SessionManager, TransportConfig,
};
use anyhow::Result;
use axum::{
    http::{Method, StatusCode},
    routing::{any, get},
    Json, Router,
};
use doc_server_database::DatabasePool;
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// MCP server state
#[derive(Clone)]
pub struct McpServerState {
    pub db_pool: DatabasePool,
    pub handler: Arc<McpHandler>,
    pub session_manager: SessionManager,
    pub transport_config: TransportConfig,
}

/// MCP server
pub struct McpServer {
    state: McpServerState,
}

impl McpServer {
    /// Create a new MCP server
    ///
    /// # Errors
    ///
    /// Returns an error if handler initialization fails.
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        let handler = Arc::new(McpHandler::new(db_pool.clone())?);

        // Initialize transport configuration
        let transport_config = TransportConfig::default();
        let session_manager = SessionManager::new(transport_config.clone());

        // Initialize the transport with session cleanup
        initialize_transport(session_manager.clone()).await;

        let state = McpServerState {
            db_pool,
            handler,
            session_manager,
            transport_config,
        };

        Ok(Self { state })
    }

    /// Start serving on the given address
    ///
    /// # Errors
    ///
    /// Returns an error if binding or serving the listener fails.
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
            // Unified MCP endpoint using new Streamable HTTP transport
            // Supports POST (JSON-RPC) and GET (SSE) - MVP: POST only with 405 for GET
            .route("/mcp", any(unified_mcp_handler))
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

// Old handlers removed - now using unified_mcp_handler from transport module

//! MCP server implementation

use anyhow::Result;
use doc_server_database::DatabasePool;
use crate::handlers::McpHandler;
use axum::{
    extract::State,
    http::{Method, StatusCode},
    response::{sse::Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use serde_json::Value;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::time::interval;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error, debug};

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
        let state = McpServerState {
            db_pool,
            handler,
        };
        
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
            // MCP SSE endpoint for real-time communication
            .route("/sse", get(sse_handler))
            // MCP JSON-RPC endpoint for tool calls
            .route("/mcp", post(mcp_handler))
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

/// SSE endpoint for real-time communication
async fn sse_handler(
    State(_state): State<McpServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    debug!("New SSE connection established");
    
    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok(Event::default()
            .event("connected")
            .data("{\"status\":\"connected\",\"server\":\"doc-server-mcp\"}")
        );
        
        // Keep-alive heartbeat every 30 seconds
        let mut heartbeat = interval(Duration::from_secs(30));
        
        loop {
            heartbeat.tick().await;
            yield Ok(Event::default()
                .event("heartbeat")
                .data(format!("{{\"timestamp\":{}}}", chrono::Utc::now().timestamp()))
            );
        }
    };
    
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive-text"),
        )
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
            Ok(Json(response))
        }
        Err(e) => {
            error!("MCP request failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
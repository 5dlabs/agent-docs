//! MCP server implementation

use crate::handlers::McpHandler;
use crate::health::{create_health_router, init_service_start_time};
use crate::security::{validate_server_binding, SecurityConfig};
use crate::session::{SessionConfig, SessionManager as ComprehensiveSessionManager};
use crate::transport::{
    initialize_transport, unified_mcp_handler, SessionManager, TransportConfig,
};
use anyhow::Result;
use axum::{http::Method, routing::any, Router};
use doc_server_database::DatabasePool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

/// MCP server state
#[derive(Clone)]
pub struct McpServerState {
    pub db_pool: DatabasePool,
    pub handler: Arc<McpHandler>,
    pub session_manager: SessionManager, // Legacy session manager for compatibility
    pub comprehensive_session_manager: ComprehensiveSessionManager, // New comprehensive session manager
    pub transport_config: TransportConfig,
    pub security_config: SecurityConfig,
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
        // Initialize service start time for uptime tracking
        init_service_start_time();
        let handler = Arc::new(McpHandler::new(db_pool.clone())?);

        // Initialize transport configuration
        let transport_config = TransportConfig::default();
        let session_manager = SessionManager::new(transport_config.clone());

        // Initialize comprehensive session manager
        let session_config = SessionConfig::default();
        let comprehensive_session_manager = ComprehensiveSessionManager::new(session_config);

        // Start background cleanup task for comprehensive session manager
        comprehensive_session_manager.start_cleanup_task();

        // Initialize security configuration
        let security_config = SecurityConfig::default();

        // Initialize the transport with legacy session cleanup (for backward compatibility)
        initialize_transport(session_manager.clone()).await;

        let state = McpServerState {
            db_pool: db_pool.clone(),
            handler,
            session_manager,
            comprehensive_session_manager,
            transport_config,
            security_config,
        };

        // Start background monitoring for the database pool
        db_pool.start_monitoring();

        Ok(Self { state })
    }

    /// Start serving on the given address
    ///
    /// # Errors
    ///
    /// Returns an error if binding or serving the listener fails, or if security validation fails.
    pub async fn serve(&self, addr: &str) -> Result<()> {
        // Validate server binding for security
        if let Err(e) = validate_server_binding(addr, &self.state.security_config) {
            error!("Server binding security validation failed: {}", e);
            return Err(anyhow::anyhow!("Security validation failed: {}", e));
        }

        let app = self.create_router();

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("MCP server listening on {} (security validated)", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create the router with all endpoints
    pub fn create_router(&self) -> Router {
        Router::new()
            // Enhanced health check endpoints
            .merge(create_health_router())
            // Unified MCP endpoint using new Streamable HTTP transport
            // Supports POST (JSON-RPC) and GET (SSE) - MVP: POST only with 405 for GET
            .route("/mcp", any(unified_mcp_handler))
            // Add CORS for Toolman compatibility
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
                    .allow_headers(Any),
            )
            .with_state(self.state.clone())
    }
}

// Old basic health check removed - now using comprehensive health endpoints from health module
// Old handlers removed - now using unified_mcp_handler from transport module

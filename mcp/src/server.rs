//! MCP server implementation

use crate::handlers::McpHandler;
use crate::health::{create_health_router, init_service_start_time};
use crate::ingest::IngestJobManager;
use crate::security::{validate_server_binding, SecurityConfig};
use crate::session::{SessionConfig, SessionManager as ComprehensiveSessionManager};
use crate::transport::{
    initialize_transport, unified_mcp_handler, SessionManager, TransportConfig,
};
use anyhow::Result;
use axum::{http::Method, routing::any, routing::post, Router};
use db::DatabasePool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

/// MCP server state
#[derive(Clone)]
pub struct McpServerState {
    pub db_pool: DatabasePool,
    pub handler: Arc<McpHandler>,
    pub session_manager: SessionManager, // Legacy session manager for compatibility
    pub comprehensive_session_manager: ComprehensiveSessionManager, // New comprehensive session manager
    pub transport_config: TransportConfig,
    pub security_config: SecurityConfig,
    pub ingest_jobs: IngestJobManager,
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
        let handler = Arc::new(McpHandler::new(&db_pool)?);

        // Initialize transport configuration
        let transport_config = TransportConfig::default();
        let session_manager = SessionManager::new(transport_config.clone());

        // Initialize comprehensive session manager
        let session_config = SessionConfig::default();
        let comprehensive_session_manager = ComprehensiveSessionManager::new(session_config);

        // Start background cleanup task for comprehensive session manager
        comprehensive_session_manager.start_cleanup_task();

        // Initialize security configuration (allow env overrides for production)
        let security_config = SecurityConfig::from_env();

        // Initialize the transport with legacy session cleanup (for backward compatibility)
        initialize_transport(session_manager.clone()).await;

        let ingest_jobs = IngestJobManager::new(db_pool.clone());
        // Start background cleanup for ingest jobs
        ingest_jobs.start_cleanup_task();

        let state = McpServerState {
            db_pool: db_pool.clone(),
            handler,
            session_manager,
            comprehensive_session_manager,
            transport_config,
            security_config,
            ingest_jobs,
        };

        // Start background monitoring for the database pool
        db_pool.start_monitoring();

        // Attempt recovery of any stale running jobs from previous restarts
        if let Err(e) = recover_stale_jobs(db_pool.pool()).await {
            warn!("Job recovery on startup encountered an error: {}", e);
        }

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
            // Intelligent ingest endpoints
            .route(
                "/ingest/intelligent",
                post(crate::ingest::intelligent_ingest_handler),
            )
            .route(
                "/ingest/jobs/{job_id}",
                axum::routing::get(crate::ingest::get_ingest_status_handler),
            )
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

/// Recover stale running jobs that may have been abandoned due to a restart
///
/// This marks jobs in 'running' state whose `updated_at` is older than a threshold
/// as failed and sets `finished_at`, preserving a clear error reason.
async fn recover_stale_jobs(pool: &sqlx::PgPool) -> Result<()> {
    // Thresholds can be tuned; keeping conservative defaults
    // Crate jobs: 30 minutes
    let crate_recovery = sqlx::query(
        r"
        UPDATE crate_jobs
        SET status = 'failed',
            finished_at = CURRENT_TIMESTAMP,
            error = COALESCE(error, '') || CASE WHEN error IS NULL OR error = '' THEN '' ELSE E'\n' END ||
                   'Recovery: marked failed on startup due to stale running (updated_at older than 30 minutes).'
        WHERE status = 'running'
          AND updated_at < CURRENT_TIMESTAMP - INTERVAL '30 minutes'
        ",
    )
    .execute(pool)
    .await?;

    let crate_count = crate_recovery.rows_affected();

    // Ingest jobs: 30 minutes
    let ingest_recovery = sqlx::query(
        r"
        UPDATE ingest_jobs
        SET status = 'failed',
            finished_at = CURRENT_TIMESTAMP,
            error = COALESCE(error, '') || CASE WHEN error IS NULL OR error = '' THEN '' ELSE E'\n' END ||
                   'Recovery: marked failed on startup due to stale running (updated_at older than 30 minutes).'
        WHERE status = 'running'
          AND updated_at < CURRENT_TIMESTAMP - INTERVAL '30 minutes'
        ",
    )
    .execute(pool)
    .await?;

    let ingest_count = ingest_recovery.rows_affected();

    if crate_count > 0 || ingest_count > 0 {
        info!(
            "Recovered stale jobs on startup: crate_jobs={}, ingest_jobs={}",
            crate_count, ingest_count
        );
    }

    Ok(())
}

// Old basic health check removed - now using comprehensive health endpoints from health module
// Old handlers removed - now using unified_mcp_handler from transport module

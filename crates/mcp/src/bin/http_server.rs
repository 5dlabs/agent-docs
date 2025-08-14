//! HTTP server binary for the Doc Server
//!
//! This binary provides the main HTTP/SSE endpoint for MCP communication.

use anyhow::Result;
use doc_server_database::{migrations::Migrations, DatabasePool};
use doc_server_mcp::McpServer;
use dotenvy::dotenv;
use std::env;
use tokio::signal;
use tracing::{error, info, warn};
//use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Check for health check argument
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "--health-check" || args[1] == "--version") {
        // For health check, just exit successfully to indicate the binary is working
        println!("doc-server v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG").unwrap_or_else(|_| "info,doc_server=debug".to_string()),
        )
        .init();

    info!("Starting Doc Server HTTP server...");

    // Get configuration from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    // Initialize database
    let db_pool = DatabasePool::new(&database_url).await?;

    // Run migrations
    if let Err(e) = Migrations::run(db_pool.pool()).await {
        error!("Failed to run migrations: {}", e);
        return Err(e);
    }

    // Initialize MCP server
    let mcp_server = McpServer::new(db_pool).await?;

    // Start HTTP server with graceful shutdown
    let addr = format!("0.0.0.0:{port}");
    info!("Doc Server listening on {}", addr);

    run_server_with_graceful_shutdown(mcp_server, &addr).await?;

    Ok(())
}

/// Run the server with graceful shutdown signal handling
async fn run_server_with_graceful_shutdown(mcp_server: McpServer, addr: &str) -> Result<()> {
    use tokio::net::TcpListener;

    // Create router and bind listener
    let app = mcp_server.create_router();
    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on {} with graceful shutdown enabled", addr);

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Wait for shutdown signal (SIGTERM or SIGINT)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            warn!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
        },
        () = terminate => {
            warn!("Received SIGTERM, initiating graceful shutdown...");
        }
    }

    info!("Shutdown signal received, starting graceful shutdown (timeout: 30s)");
}

//! HTTP server binary for the Doc Server
//!
//! This binary provides the main HTTP/SSE endpoint for MCP communication.

use anyhow::Result;
use doc_server_database::{migrations::Migrations, DatabasePool};
use doc_server_mcp::McpServer;
use dotenvy::dotenv;
use std::env;
use tracing::{error, info};
//use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
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

    // Start HTTP server
    let addr = format!("0.0.0.0:{}", port);
    info!("Doc Server listening on {}", addr);

    mcp_server.serve(&addr).await?;

    Ok(())
}

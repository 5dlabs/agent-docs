//! MCP server implementation

use anyhow::Result;
use doc_server_database::DatabasePool;

/// MCP server
pub struct McpServer {
    _db_pool: DatabasePool,
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        Ok(Self { _db_pool: db_pool })
    }
    
    /// Start serving on the given address
    pub async fn serve(&self, _addr: &str) -> Result<()> {
        // TODO: Implement HTTP/SSE server
        tracing::info!("MCP server would start here");
        Ok(())
    }
}
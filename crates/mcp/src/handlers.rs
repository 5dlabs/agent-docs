//! MCP request handlers

use crate::tools::{RustQueryTool, Tool};
use anyhow::{anyhow, Result};
use doc_server_database::DatabasePool;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, error};

/// MCP request handler
pub struct McpHandler {
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl McpHandler {
    /// Create a new MCP handler
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();

        // Register the rust_query tool
        let rust_query_tool = RustQueryTool::new(db_pool).await?;
        tools.insert("rust_query".to_string(), Box::new(rust_query_tool));

        Ok(Self { tools })
    }

    /// Handle an MCP request
    pub async fn handle_request(&self, request: Value) -> Result<Value> {
        debug!("Processing MCP request");

        // Extract method from request
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing method in request"))?;

        match method {
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(&request).await,
            "initialize" => self.handle_initialize(&request).await,
            _ => Err(anyhow!("Unsupported method: {}", method)),
        }
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self) -> Result<Value> {
        let tools: Vec<Value> = self.tools.values().map(|tool| tool.definition()).collect();

        Ok(json!({
            "tools": tools
        }))
    }

    /// Handle tools/call request
    async fn handle_tool_call(&self, request: &Value) -> Result<Value> {
        let params = request
            .get("params")
            .ok_or_else(|| anyhow!("Missing params in tool call"))?;

        let tool_name = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Missing tool name"))?;

        let default_args = json!({});
        let arguments = params.get("arguments").unwrap_or(&default_args);

        debug!("Calling tool: {} with arguments: {}", tool_name, arguments);

        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?;

        match tool.execute(arguments.clone()).await {
            Ok(result) => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": result
                    }
                ]
            })),
            Err(e) => {
                error!("Tool execution failed: {}", e);
                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Error: {}", e)
                        }
                    ],
                    "isError": true
                }))
            }
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, _request: &Value) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "doc-server-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }
}

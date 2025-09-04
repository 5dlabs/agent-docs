//! MCP request handlers

use crate::config::ConfigLoader;
use crate::crate_tools::{
    AddRustCrateTool, CheckRustStatusTool, ListRustCratesTool, RemoveRustCrateTool,
};
use crate::protocol_version::ProtocolRegistry;
use crate::tools::{DynamicQueryTool, RustQueryTool, Tool};
use anyhow::{anyhow, Result};
use db::DatabasePool;
use embed::OpenAIEmbeddingClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// MCP request handler
pub struct McpHandler {
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl McpHandler {
    /// Create a new MCP handler
    ///
    /// # Errors
    ///
    /// Returns an error if any tool initialization fails.
    pub fn new(db_pool: &DatabasePool) -> Result<Self> {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();

        // Always register the rust_query tool as hardcoded (legacy)
        let rust_query_tool = RustQueryTool::new(db_pool.clone())?;
        tools.insert("rust_query".to_string(), Box::new(rust_query_tool));
        debug!("Registered hardcoded rust_query tool");

        // Register server-side ingest tool (spawns loader CLI)
        tools.insert("ingest".to_string(), Box::new(crate::tools::IngestTool::new()));
        debug!("Registered hardcoded ingest tool");

        // Load and register dynamic tools from configuration
        match Self::register_dynamic_tools(&mut tools, db_pool) {
            Ok(count) => {
                info!(
                    "Successfully registered {} dynamic tools from configuration",
                    count
                );
            }
            Err(e) => {
                warn!(
                    "Failed to load dynamic tools: {}. Continuing with hardcoded tools only.",
                    e
                );
            }
        }

        info!("MCP handler initialized with {} total tools", tools.len());
        Ok(Self { tools })
    }

    /// Register dynamic tools from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or tool creation fails.
    fn register_dynamic_tools(
        tools: &mut HashMap<String, Box<dyn Tool + Send + Sync>>,
        db_pool: &DatabasePool,
    ) -> Result<usize> {
        // First try to load from config file, fall back to embedded config
        let config = if let Ok(path) = std::env::var("TOOLS_CONFIG_PATH") {
            info!("Loading tools configuration from: {path}");
            ConfigLoader::load_from_file(path)?
        } else {
            debug!("No TOOLS_CONFIG_PATH specified, using embedded configuration");
            ConfigLoader::load_default()?
        };

        let enabled_tools = ConfigLoader::filter_enabled_tools(&config);
        let mut registered_count = 0;

        for tool_config in enabled_tools {
            // Skip rust_query if it appears in config since we register it hardcoded
            if tool_config.name == "rust_query" {
                debug!("Skipping rust_query from config - already registered as hardcoded");
                continue;
            }

            // Check if tool name already exists
            if tools.contains_key(&tool_config.name) {
                warn!("Tool '{}' already registered, skipping", tool_config.name);
                continue;
            }

            // Create and register the tool based on tool name
            match Self::create_tool_from_config(&tool_config, db_pool) {
                Ok(tool) => {
                    debug!(
                        "Created dynamic tool '{}' for doc_type '{}'",
                        tool_config.name, tool_config.doc_type
                    );
                    tools.insert(tool_config.name.clone(), tool);
                    registered_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to create tool '{}': {}. Skipping.",
                        tool_config.name, e
                    );
                }
            }
        }

        Ok(registered_count)
    }

    /// Create a tool instance from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if tool creation fails.
    fn create_tool_from_config(
        tool_config: &db::models::ToolConfig,
        db_pool: &DatabasePool,
    ) -> Result<Box<dyn Tool + Send + Sync>> {
        match tool_config.name.as_str() {
            // Crate management tools
            "add_rust_crate" => {
                let embedding_client: Arc<dyn embed::client::EmbeddingClient + Send + Sync> =
                    Arc::new(OpenAIEmbeddingClient::new()?);
                Ok(Box::new(AddRustCrateTool::new(
                    db_pool.clone(),
                    embedding_client,
                )))
            }
            "remove_rust_crate" => Ok(Box::new(RemoveRustCrateTool::new(db_pool.clone()))),
            "list_rust_crates" => Ok(Box::new(ListRustCratesTool::new(db_pool.clone()))),
            "check_rust_status" => Ok(Box::new(CheckRustStatusTool::new(db_pool.clone()))),
            // Query tools - use the existing dynamic pattern
            _ => Ok(Box::new(DynamicQueryTool::new(
                tool_config.clone(),
                db_pool.clone(),
            )?)),
        }
    }

    /// Handle an MCP request
    ///
    /// # Errors
    ///
    /// Returns an error when the request is malformed or tool execution fails.
    pub async fn handle_request(&self, request: Value) -> Result<Value> {
        debug!("Processing MCP request");

        // Extract method from request
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing method in request"))?;

        match method {
            "tools/list" => Ok(self.handle_tools_list()),
            "tools/call" => self.handle_tool_call(&request).await,
            "initialize" => Ok(Self::handle_initialize(&request)),
            _ => Err(anyhow!("Unsupported method: {}", method)),
        }
    }

    /// Handle tools/list request
    fn handle_tools_list(&self) -> Value {
        let tools: Vec<Value> = self.tools.values().map(|tool| tool.definition()).collect();

        json!({
            "tools": tools
        })
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
    ///
    /// Returns the initialization result with the fixed protocol version
    /// and server capabilities.
    fn handle_initialize(_request: &Value) -> Value {
        let registry = ProtocolRegistry::new();

        debug!(
            "Handling initialize request with protocol version: {}",
            registry.current_version_string()
        );

        json!({
            "protocolVersion": registry.current_version_string(),
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })
    }
}

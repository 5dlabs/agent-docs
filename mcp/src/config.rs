//! Configuration loading and validation

use anyhow::{anyhow, Result};
use db::models::{ToolConfig, ToolsConfig};
use std::path::Path;
use tracing::{debug, info};

/// Configuration loader for dynamic tools
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load tools configuration from JSON file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is not valid JSON,
    /// or doesn't match the expected schema.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<ToolsConfig> {
        debug!("Loading configuration from: {:?}", path.as_ref());

        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow!("Failed to read config file {:?}: {}", path.as_ref(), e))?;

        let config: ToolsConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config JSON: {}", e))?;

        Self::validate_config(&config)?;

        info!(
            "Loaded configuration with {} tools from {:?}",
            config.tools.len(),
            path.as_ref()
        );

        Ok(config)
    }

    /// Load tools configuration from filesystem or embedded fallback
    ///
    /// # Errors
    ///
    /// Returns an error if the config is invalid.
    pub fn load_default() -> Result<ToolsConfig> {
        // Try to load from environment variable first (highest priority)
        if let Ok(config_content) = std::env::var("TOOLS_CONFIG") {
            debug!("Loading configuration from environment variable TOOLS_CONFIG");

            let config: ToolsConfig = serde_json::from_str(&config_content)
                .map_err(|e| anyhow!("Failed to parse environment config: {}", e))?;

            Self::validate_config(&config)?;

            info!(
                "Loaded environment configuration with {} tools",
                config.tools.len()
            );

            return Ok(config);
        }

        // Try to load from filesystem second (expected in production)
        match std::fs::read_to_string("/app/tools.json") {
            Ok(config_content) => {
                debug!("Loading configuration from filesystem (/app/tools.json)");
                info!(
                    "Successfully read {} bytes from filesystem",
                    config_content.len()
                );

                let config: ToolsConfig = serde_json::from_str(&config_content)
                    .map_err(|e| anyhow!("Failed to parse filesystem config: {}", e))?;

                Self::validate_config(&config)?;

                info!(
                    "Loaded filesystem configuration with {} tools",
                    config.tools.len()
                );

                Ok(config)
            }
            Err(e) => {
                // No fallback to embedded config - configuration is now required
                debug!(
                    "Failed to load tools configuration from /app/tools.json: {}",
                    e
                );
                Err(anyhow!(
                    "No tools configuration found. Configuration must be provided via:\n\
                    1. TOOLS_CONFIG environment variable, or\n\
                    2. /app/tools.json file (mounted from ConfigMap)\n\
                    Error: {}",
                    e
                ))
            }
        }
    }

    /// Validate the tools configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate_config(config: &ToolsConfig) -> Result<()> {
        if config.tools.is_empty() {
            return Err(anyhow!("Configuration must contain at least one tool"));
        }

        let mut tool_names = std::collections::HashSet::new();

        for tool in &config.tools {
            // Check for empty fields
            if tool.name.is_empty() {
                return Err(anyhow!("Tool name cannot be empty"));
            }
            if tool.doc_type.is_empty() {
                return Err(anyhow!("Tool docType cannot be empty"));
            }
            if tool.title.is_empty() {
                return Err(anyhow!("Tool title cannot be empty"));
            }
            if tool.description.is_empty() {
                return Err(anyhow!("Tool description cannot be empty"));
            }

            // Check for unique tool names
            if !tool_names.insert(&tool.name) {
                return Err(anyhow!("Duplicate tool name: {}", tool.name));
            }

            // Validate tool name format - allow both query tools and management tools
            let is_query_tool = tool.name.ends_with("_query");
            let is_crate_management_tool = matches!(
                tool.name.as_str(),
                "add_rust_crate" | "remove_rust_crate" | "list_rust_crates" | "check_rust_status"
            );

            if !is_query_tool && !is_crate_management_tool {
                return Err(anyhow!(
                    "Tool name '{}' must either end with '_query' or be a valid crate management tool (add_rust_crate, remove_rust_crate, list_rust_crates, check_rust_status)", 
                    tool.name
                ));
            }

            // Validate doc_type - all doc types from the configuration are considered valid
            // No need to validate against a hardcoded list since we extract them dynamically

            debug!("Validated tool: {} -> {}", tool.name, tool.doc_type);
        }

        Ok(())
    }

    /// Filter enabled tools from configuration
    #[must_use]
    pub fn filter_enabled_tools(config: &ToolsConfig) -> Vec<ToolConfig> {
        config
            .tools
            .iter()
            .filter(|tool| tool.enabled)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        // Test 1: Loading without config file or env var should fail
        // Save current env var if it exists
        let saved_env = std::env::var("TOOLS_CONFIG").ok();
        std::env::remove_var("TOOLS_CONFIG");

        let result = ConfigLoader::load_default();
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No tools configuration found"));
        assert!(error_msg.contains("TOOLS_CONFIG environment variable"));
        assert!(error_msg.contains("/app/tools.json"));

        // Test 2: Loading configuration from environment variable should work
        let test_config = r#"{
            "tools": [
                {
                    "name": "test_query",
                    "docType": "test",
                    "title": "Test Query Tool",
                    "description": "A test query tool",
                    "enabled": true
                }
            ]
        }"#;

        std::env::set_var("TOOLS_CONFIG", test_config);

        let config = ConfigLoader::load_default().expect("Should load config from env var");
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].name, "test_query");

        // Restore original env var state
        match saved_env {
            Some(val) => std::env::set_var("TOOLS_CONFIG", val),
            None => std::env::remove_var("TOOLS_CONFIG"),
        }
    }

    #[test]
    fn test_validate_config_empty_tools() {
        let config = ToolsConfig { tools: vec![] };
        let result = ConfigLoader::validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one tool"));
    }

    #[test]
    fn test_validate_config_duplicate_names() {
        let tool1 = ToolConfig {
            name: "test_query".to_string(),
            doc_type: "rust".to_string(),
            title: "Test".to_string(),
            description: "Test tool".to_string(),
            enabled: true,
            metadata_hints: None,
        };
        let tool2 = tool1.clone();
        let config = ToolsConfig {
            tools: vec![tool1, tool2],
        };

        let result = ConfigLoader::validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));
    }

    #[test]
    fn test_validate_config_invalid_name_format() {
        let tool = ToolConfig {
            name: "test_tool".to_string(), // Should end with "_query"
            doc_type: "rust".to_string(),
            title: "Test".to_string(),
            description: "Test tool".to_string(),
            enabled: true,
            metadata_hints: None,
        };
        let config = ToolsConfig { tools: vec![tool] };

        let result = ConfigLoader::validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must either end with '_query' or be a valid crate management tool"));
    }

    #[test]
    fn test_validate_config_invalid_doc_type() {
        let tool = ToolConfig {
            name: "test_query".to_string(),
            doc_type: String::new(), // Empty doc type is invalid
            title: "Test".to_string(),
            description: "Test tool".to_string(),
            enabled: true,
            metadata_hints: None,
        };
        let config = ToolsConfig { tools: vec![tool] };

        let result = ConfigLoader::validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("docType cannot be empty"));
    }

    #[test]
    fn test_filter_enabled_tools() {
        let tool1 = ToolConfig {
            name: "enabled_query".to_string(),
            doc_type: "rust".to_string(),
            title: "Enabled".to_string(),
            description: "Enabled tool".to_string(),
            enabled: true,
            metadata_hints: None,
        };
        let tool2 = ToolConfig {
            name: "disabled_query".to_string(),
            doc_type: "solana".to_string(),
            title: "Disabled".to_string(),
            description: "Disabled tool".to_string(),
            enabled: false,
            metadata_hints: None,
        };
        let config = ToolsConfig {
            tools: vec![tool1, tool2],
        };

        let enabled = ConfigLoader::filter_enabled_tools(&config);
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "enabled_query");
    }
}

//! Tests for dynamic tool metadata filtering and formatting

use doc_server_database::models::{ToolMetadataHints, ToolsConfig};
use doc_server_database::queries::MetadataFilters;
use doc_server_mcp::config::ConfigLoader;
use serde_json::json;

#[test]
fn test_metadata_hints_in_config() {
    // Test configuration with metadata hints
    let config_with_hints = json!({
        "tools": [
            {
                "name": "solana_query",
                "docType": "solana",
                "title": "Solana Documentation Query",
                "description": "Search Solana core documentation with advanced filtering",
                "enabled": true,
                "metadataHints": {
                    "supported_formats": ["markdown", "pdf", "bob", "msc"],
                    "supported_complexity_levels": ["beginner", "intermediate", "advanced"],
                    "supported_categories": ["core", "architecture", "zk-cryptography"],
                    "supported_topics": ["consensus", "networking", "validators"],
                    "supports_api_version": false
                }
            },
            {
                "name": "birdeye_query",
                "docType": "birdeye",
                "title": "BirdEye API Query",
                "description": "Query BirdEye API documentation",
                "enabled": true,
                "metadataHints": {
                    "supported_formats": ["json", "yaml"],
                    "supported_complexity_levels": [],
                    "supported_categories": ["defi", "pricing", "token"],
                    "supported_topics": ["api", "endpoints"],
                    "supports_api_version": true
                }
            }
        ]
    });

    let parsed_config: ToolsConfig = serde_json::from_value(config_with_hints)
        .expect("Should parse configuration with metadata hints");

    // Validate the configuration
    let validation_result = ConfigLoader::validate_config(&parsed_config);
    assert!(
        validation_result.is_ok(),
        "Configuration with metadata hints should be valid"
    );

    // Check Solana tool metadata hints
    let solana_tool = parsed_config
        .tools
        .iter()
        .find(|tool| tool.name == "solana_query")
        .expect("Should have solana_query tool");

    assert!(
        solana_tool.metadata_hints.is_some(),
        "Solana tool should have metadata hints"
    );

    let solana_hints = solana_tool.metadata_hints.as_ref().unwrap();
    assert_eq!(solana_hints.supported_formats.len(), 4);
    assert!(solana_hints.supported_formats.contains(&"bob".to_string()));
    assert!(solana_hints.supported_formats.contains(&"msc".to_string()));
    assert_eq!(solana_hints.supported_complexity_levels.len(), 3);
    assert!(!solana_hints.supports_api_version);

    // Check BirdEye tool metadata hints
    let birdeye_tool = parsed_config
        .tools
        .iter()
        .find(|tool| tool.name == "birdeye_query")
        .expect("Should have birdeye_query tool");

    let birdeye_hints = birdeye_tool.metadata_hints.as_ref().unwrap();
    assert!(birdeye_hints.supports_api_version);
    assert!(birdeye_hints
        .supported_categories
        .contains(&"defi".to_string()));
}

#[test]
fn test_metadata_filters_struct() {
    // Test MetadataFilters struct functionality
    let mut filters = MetadataFilters::default();
    assert!(filters.format.is_none());
    assert!(filters.complexity.is_none());
    assert!(filters.category.is_none());
    assert!(filters.topic.is_none());
    assert!(filters.api_version.is_none());

    // Test setting filters
    filters.format = Some("pdf".to_string());
    filters.complexity = Some("advanced".to_string());
    filters.category = Some("architecture".to_string());
    filters.topic = Some("consensus".to_string());
    filters.api_version = Some("v2".to_string());

    assert_eq!(filters.format.as_ref().unwrap(), "pdf");
    assert_eq!(filters.complexity.as_ref().unwrap(), "advanced");
    assert_eq!(filters.category.as_ref().unwrap(), "architecture");
    assert_eq!(filters.topic.as_ref().unwrap(), "consensus");
    assert_eq!(filters.api_version.as_ref().unwrap(), "v2");
}

#[test]
fn test_config_without_metadata_hints() {
    // Test that tools without metadata hints work correctly
    let config_without_hints = json!({
        "tools": [
            {
                "name": "simple_query",
                "docType": "rust",
                "title": "Simple Query Tool",
                "description": "A simple tool without metadata hints",
                "enabled": true
            }
        ]
    });

    let parsed_config: ToolsConfig = serde_json::from_value(config_without_hints)
        .expect("Should parse configuration without metadata hints");

    let simple_tool = &parsed_config.tools[0];
    assert!(simple_tool.metadata_hints.is_none());

    // Validate the configuration
    let validation_result = ConfigLoader::validate_config(&parsed_config);
    assert!(
        validation_result.is_ok(),
        "Configuration without metadata hints should be valid"
    );
}

#[test]
fn test_tool_metadata_hints_creation() {
    let hints = ToolMetadataHints {
        supported_formats: vec!["markdown".to_string(), "pdf".to_string()],
        supported_complexity_levels: vec!["beginner".to_string(), "advanced".to_string()],
        supported_categories: vec!["docs".to_string(), "guides".to_string()],
        supported_topics: vec!["installation".to_string(), "configuration".to_string()],
        supports_api_version: false,
    };

    assert_eq!(hints.supported_formats.len(), 2);
    assert_eq!(hints.supported_complexity_levels.len(), 2);
    assert_eq!(hints.supported_categories.len(), 2);
    assert_eq!(hints.supported_topics.len(), 2);
    assert!(!hints.supports_api_version);
}

#[test]
fn test_mixed_metadata_hints_config() {
    // Test configuration with mixed metadata hints (some tools have them, some don't)
    let mixed_config = json!({
        "tools": [
            {
                "name": "with_hints_query",
                "docType": "solana",
                "title": "Tool with Hints",
                "description": "Tool that has metadata hints",
                "enabled": true,
                "metadataHints": {
                    "supported_formats": ["markdown"],
                    "supported_complexity_levels": ["beginner"],
                    "supported_categories": ["docs"],
                    "supported_topics": ["basics"],
                    "supports_api_version": false
                }
            },
            {
                "name": "without_hints_query",
                "docType": "rust",
                "title": "Tool without Hints",
                "description": "Tool that has no metadata hints",
                "enabled": true
            }
        ]
    });

    let parsed_config: ToolsConfig =
        serde_json::from_value(mixed_config).expect("Should parse mixed configuration");

    assert_eq!(parsed_config.tools.len(), 2);

    let with_hints = &parsed_config.tools[0];
    let without_hints = &parsed_config.tools[1];

    assert!(with_hints.metadata_hints.is_some());
    assert!(without_hints.metadata_hints.is_none());

    // Validate the configuration
    let validation_result = ConfigLoader::validate_config(&parsed_config);
    assert!(
        validation_result.is_ok(),
        "Mixed configuration should be valid"
    );
}

#[test]
fn test_empty_metadata_hints() {
    // Test that empty metadata hints are handled correctly
    let empty_hints_config = json!({
        "tools": [
            {
                "name": "empty_hints_query",
                "docType": "rust",
                "title": "Tool with Empty Hints",
                "description": "Tool that has empty metadata hints",
                "enabled": true,
                "metadataHints": {
                    "supported_formats": [],
                    "supported_complexity_levels": [],
                    "supported_categories": [],
                    "supported_topics": [],
                    "supports_api_version": false
                }
            }
        ]
    });

    let parsed_config: ToolsConfig = serde_json::from_value(empty_hints_config)
        .expect("Should parse configuration with empty hints");

    let tool = &parsed_config.tools[0];
    let hints = tool.metadata_hints.as_ref().unwrap();

    assert!(hints.supported_formats.is_empty());
    assert!(hints.supported_complexity_levels.is_empty());
    assert!(hints.supported_categories.is_empty());
    assert!(hints.supported_topics.is_empty());
    assert!(!hints.supports_api_version);

    // Validate the configuration
    let validation_result = ConfigLoader::validate_config(&parsed_config);
    assert!(
        validation_result.is_ok(),
        "Configuration with empty hints should be valid"
    );
}

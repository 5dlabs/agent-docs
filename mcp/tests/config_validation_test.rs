//! Tests for configuration validation and dynamic tool setup without database

use mcp::config::ConfigLoader;
use serde_json::json;

fn setup_test_config() {
    // Load test configuration into environment variable
    let test_config = include_str!("test_config.json");
    std::env::set_var("TOOLS_CONFIG", test_config);
    // Also set an explicit file path to avoid env var races across parallel tests
    let path = format!(
        "{}/tests/test_config.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::env::set_var("TOOLS_CONFIG_PATH", path);
}

fn cleanup_test_config() {
    std::env::remove_var("TOOLS_CONFIG");
    std::env::remove_var("TOOLS_CONFIG_PATH");
}

#[tokio::test]
async fn test_configuration_validation() {
    setup_test_config();

    // Test that we can load the configuration successfully
    let config = ConfigLoader::load_default().expect("Should load configuration");

    cleanup_test_config();

    // Verify configuration structure
    assert!(
        !config.tools.is_empty(),
        "Configuration should have tools defined"
    );

    // Verify each tool has required fields
    for tool in &config.tools {
        assert!(
            !tool.name.is_empty(),
            "Tool name should not be empty: {tool:?}"
        );
        assert!(
            !tool.doc_type.is_empty(),
            "Doc type should not be empty: {tool:?}"
        );
        assert!(
            !tool.title.is_empty(),
            "Title should not be empty: {tool:?}"
        );
        assert!(
            !tool.description.is_empty(),
            "Description should not be empty: {tool:?}"
        );
        let is_valid_name = tool.name.ends_with("_query")
            || matches!(
                tool.name.as_str(),
                "add_rust_crate" | "remove_rust_crate" | "list_rust_crates" | "check_rust_status"
            );
        assert!(
            is_valid_name,
            "Tool name should end with '_query' or be a valid crate management tool: {}",
            tool.name
        );
    }

    // Verify expected dynamic tools are present
    let expected_tools = vec![
        "birdeye_query",
        "solana_query",
        "jupiter_query",
        "cilium_query",
        "talos_query",
        "meteora_query",
        "raydium_query",
        "ebpf_query",
        "rust_best_practices_query",
        "add_rust_crate",
        "remove_rust_crate",
        "list_rust_crates",
        "check_rust_status",
    ];

    for expected_tool in &expected_tools {
        let found = config.tools.iter().any(|tool| tool.name == *expected_tool);
        assert!(
            found,
            "Expected tool '{expected_tool}' not found in configuration"
        );
    }

    // Test filtering enabled tools
    let enabled_tools = ConfigLoader::filter_enabled_tools(&config);
    assert!(!enabled_tools.is_empty(), "Should have enabled tools");

    // All default tools should be enabled
    assert_eq!(
        config.tools.len(),
        enabled_tools.len(),
        "All default tools should be enabled"
    );
}

#[test]
fn test_configuration_validation_with_custom_data() {
    // Test validation with custom configuration data
    let test_config = json!({
        "tools": [
            {
                "name": "test_query",
                "docType": "rust",
                "title": "Test Documentation Query",
                "description": "Test tool description",
                "enabled": true
            },
            {
                "name": "disabled_query",
                "docType": "solana",
                "title": "Disabled Tool",
                "description": "This tool is disabled",
                "enabled": false
            }
        ]
    });

    let config: db::models::ToolsConfig =
        serde_json::from_value(test_config).expect("Should parse test config");

    // Test validation
    let validation_result = ConfigLoader::validate_config(&config);
    assert!(
        validation_result.is_ok(),
        "Valid configuration should pass validation"
    );

    // Test filtering
    let enabled_tools = ConfigLoader::filter_enabled_tools(&config);
    assert_eq!(enabled_tools.len(), 1, "Should have one enabled tool");
    assert_eq!(
        enabled_tools[0].name, "test_query",
        "Should filter correct tool"
    );
}

#[test]
fn test_configuration_validation_failures() {
    use db::models::{ToolConfig, ToolsConfig};

    // Test with invalid tool name (doesn't end with _query)
    let invalid_name_config = ToolsConfig {
        tools: vec![ToolConfig {
            name: "invalid_tool".to_string(),
            doc_type: "rust".to_string(),
            title: "Invalid Tool".to_string(),
            description: "Invalid tool name".to_string(),
            enabled: true,
            metadata_hints: None,
        }],
    };

    let result = ConfigLoader::validate_config(&invalid_name_config);
    assert!(
        result.is_err(),
        "Should fail validation for invalid tool name"
    );

    // Test with empty doc_type (truly invalid)
    let empty_doc_type_config = ToolsConfig {
        tools: vec![ToolConfig {
            name: "valid_query".to_string(),
            doc_type: String::new(), // Empty doc type is invalid
            title: "Valid Tool".to_string(),
            description: "Valid tool with empty doc type".to_string(),
            enabled: true,
            metadata_hints: None,
        }],
    };

    let result = ConfigLoader::validate_config(&empty_doc_type_config);
    assert!(result.is_err(), "Should fail validation for empty doc type");

    // Test with duplicate names
    let duplicate_names_config = ToolsConfig {
        tools: vec![
            ToolConfig {
                name: "duplicate_query".to_string(),
                doc_type: "rust".to_string(),
                title: "First Tool".to_string(),
                description: "First tool".to_string(),
                enabled: true,
                metadata_hints: None,
            },
            ToolConfig {
                name: "duplicate_query".to_string(),
                doc_type: "solana".to_string(),
                title: "Second Tool".to_string(),
                description: "Second tool with same name".to_string(),
                enabled: true,
                metadata_hints: None,
            },
        ],
    };

    let result = ConfigLoader::validate_config(&duplicate_names_config);
    assert!(
        result.is_err(),
        "Should fail validation for duplicate tool names"
    );
}

#[test]
fn test_doctype_to_tool_name_mapping() {
    setup_test_config();
    let config = ConfigLoader::load_default().expect("Should load configuration");
    cleanup_test_config();

    // Verify that tools follow proper naming conventions based on their doc type
    for tool in &config.tools {
        // For rust doc_type, supports multiple tools - both query and management
        if tool.doc_type == "rust" {
            assert!(
                matches!(
                    tool.name.as_str(),
                    "add_rust_crate"
                        | "remove_rust_crate"
                        | "list_rust_crates"
                        | "check_rust_status"
                ),
                "Unexpected rust tool name: {}",
                tool.name
            );
        } else {
            // For non-rust doc types, tools should be query tools ending with "_query"
            assert!(
                tool.name.ends_with("_query"),
                "Non-rust tool '{}' should end with '_query', but doc_type is '{}'",
                tool.name,
                tool.doc_type
            );
            // The tool name should match the doc type (with _query suffix)
            let expected_name = format!("{}_query", tool.doc_type);
            assert_eq!(
                tool.name, expected_name,
                "Tool name '{}' should match doc_type '{}' with '_query' suffix",
                tool.name, tool.doc_type
            );
        }
    }
}

#[test]
fn test_tool_description_quality() {
    setup_test_config();
    let config = ConfigLoader::load_default().expect("Should load configuration");
    cleanup_test_config();

    // Verify that all descriptions are substantial and informative
    for tool in &config.tools {
        assert!(
            tool.description.len() > 50,
            "Tool description should be substantial: {} has only {} characters",
            tool.name,
            tool.description.len()
        );

        // Check that description mentions the tool's doc type or related terms
        let description_lower = tool.description.to_lowercase();

        // At minimum, the description should mention the doc type
        let mentions_doc_type = description_lower.contains(&tool.doc_type.to_lowercase());

        assert!(
            mentions_doc_type,
            "Tool description for {} should mention its doc type '{}'. Description: {}",
            tool.name, tool.doc_type, tool.description
        );
    }
}

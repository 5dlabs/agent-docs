//! Tests for configuration validation and dynamic tool setup without database

use doc_server_mcp::config::ConfigLoader;
use serde_json::json;

#[tokio::test]
async fn test_configuration_validation() {
    // Test that we can load the default configuration successfully
    let config = ConfigLoader::load_default().expect("Should load default configuration");

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
        assert!(
            tool.name.ends_with("_query"),
            "Tool name should end with '_query': {}",
            tool.name
        );
    }

    // Verify expected dynamic tools are present
    let expected_tools = vec![
        "birdeye_query",
        "solana_query",
        "jupyter_query",
        "cilium_query",
        "talos_query",
        "meteora_query",
        "raydium_query",
        "ebpf_query",
        "rust_best_practices_query",
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

    let config: doc_server_database::models::ToolsConfig =
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
    use doc_server_database::models::{ToolConfig, ToolsConfig};

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

    // Test with invalid doc_type
    let invalid_doc_type_config = ToolsConfig {
        tools: vec![ToolConfig {
            name: "valid_query".to_string(),
            doc_type: "invalid_type".to_string(),
            title: "Valid Tool".to_string(),
            description: "Valid tool with invalid doc type".to_string(),
            enabled: true,
            metadata_hints: None,
        }],
    };

    let result = ConfigLoader::validate_config(&invalid_doc_type_config);
    assert!(
        result.is_err(),
        "Should fail validation for invalid doc type"
    );

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
    let config = ConfigLoader::load_default().expect("Should load default configuration");

    // Verify that each docType maps to an appropriately named tool
    for tool in &config.tools {
        match tool.doc_type.as_str() {
            "birdeye" => assert_eq!(tool.name, "birdeye_query"),
            "solana" => assert_eq!(tool.name, "solana_query"),
            "jupyter" => assert_eq!(tool.name, "jupyter_query"),
            "cilium" => assert_eq!(tool.name, "cilium_query"),
            "talos" => assert_eq!(tool.name, "talos_query"),
            "meteora" => assert_eq!(tool.name, "meteora_query"),
            "raydium" => assert_eq!(tool.name, "raydium_query"),
            "ebpf" => assert_eq!(tool.name, "ebpf_query"),
            "rust_best_practices" => assert_eq!(tool.name, "rust_best_practices_query"),
            _ => panic!("Unexpected doc_type: {}", tool.doc_type),
        }
    }
}

#[test]
fn test_tool_description_quality() {
    let config = ConfigLoader::load_default().expect("Should load default configuration");

    // Verify that all descriptions are substantial and informative
    for tool in &config.tools {
        assert!(
            tool.description.len() > 50,
            "Tool description should be substantial: {} has only {} characters",
            tool.name,
            tool.description.len()
        );

        // Check that description mentions the tool's domain
        let description_lower = tool.description.to_lowercase();
        let doc_type_variants = match tool.doc_type.as_str() {
            "birdeye" => vec!["birdeye", "blockchain", "api"],
            "solana" => vec!["solana", "blockchain", "validator"],
            "jupyter" => vec!["jupyter", "notebook", "data"],
            "cilium" => vec!["cilium", "networking", "kubernetes"],
            "talos" => vec!["talos", "kubernetes", "linux"],
            "meteora" => vec!["meteora", "defi", "protocol"],
            "raydium" => vec!["raydium", "dex", "amm"],
            "ebpf" => vec!["ebpf", "kernel", "filter"],
            "rust_best_practices" => vec!["rust", "practices", "patterns"],
            _ => vec![],
        };

        let mentions_domain = doc_type_variants
            .iter()
            .any(|variant| description_lower.contains(variant));

        assert!(
            mentions_domain,
            "Tool description for {} should mention its domain. Description: {}",
            tool.name, tool.description
        );
    }
}

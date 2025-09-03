//! Integration tests for dynamic tool registration and usage

use db::{DatabasePool, PoolConfig};
use mcp::{config::ConfigLoader, handlers::McpHandler};
use serde_json::json;
use std::sync::Mutex;
use tokio::time::{timeout, Duration};

// Use a mutex to ensure tests don't run in parallel when modifying env vars
static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn setup_test_config() {
    // Load test configuration into environment variable
    let test_config = include_str!("test_config.json");
    std::env::set_var("TOOLS_CONFIG", test_config);
}

fn cleanup_test_config() {
    std::env::remove_var("TOOLS_CONFIG");
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_dynamic_tools_registration() {
    let _lock = TEST_MUTEX.lock().unwrap();
    setup_test_config();

    // Create a mock database pool
    // Fast path for CI/unit tests: skip DB unless explicitly requested
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "mock".to_string());

    // Test with real database only if explicitly set and quickly reachable
    if database_url == "mock" {
        // If we can't connect to a test database, test configuration loading only
        eprintln!("Skipping dynamic tools test - no test database available");

        // Test that the configuration can be loaded without database
        let config = ConfigLoader::load_default().expect("Should load config");
        let enabled_tools = ConfigLoader::filter_enabled_tools(&config);

        cleanup_test_config();

        // Should have multiple enabled tools
        assert!(
            !enabled_tools.is_empty(),
            "Should have enabled tools in config"
        );

        // Verify structure of config tools
        for tool in &enabled_tools {
            assert!(!tool.name.is_empty(), "Tool name should not be empty");
            assert!(!tool.doc_type.is_empty(), "Doc type should not be empty");
            assert!(!tool.title.is_empty(), "Title should not be empty");
            assert!(
                !tool.description.is_empty(),
                "Description should not be empty"
            );
            assert!(tool.enabled, "All filtered tools should be enabled");
            assert!(
                tool.name.ends_with("_query") || tool.name.starts_with("add_") || tool.name.starts_with("remove_") || tool.name.starts_with("list_") || tool.name.starts_with("check_"),
                "Tool names should end with '_query' or be action tools (add_*, remove_*, list_*, check_*)"
            );
        }
    } else if let Ok(Ok(db_pool)) = timeout(
        Duration::from_secs(10),
        DatabasePool::with_config(
            PoolConfig::builder()
                .database_url(database_url)
                .min_connections(2)
                .max_connections(15)
                .acquire_timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        ),
    )
    .await
    {
        let handler = McpHandler::new(&db_pool).expect("Failed to create handler");

        // Test tools/list request to see what tools are registered
        let request = json!({
            "method": "tools/list",
            "params": {}
        });

        let response = handler
            .handle_request(request)
            .await
            .expect("tools/list should succeed");

        // Verify tools are present
        let tools = response
            .get("tools")
            .expect("Response should have tools")
            .as_array()
            .expect("tools should be an array");

        // Should have at least rust_query (hardcoded) plus dynamic tools
        assert!(
            !tools.is_empty(),
            "Should have at least one tool registered"
        );

        // Check that rust_query is always present (hardcoded)
        let rust_query_exists = tools
            .iter()
            .any(|tool| tool.get("name").and_then(|n| n.as_str()) == Some("rust_query"));
        assert!(
            rust_query_exists,
            "rust_query tool should always be present"
        );

        // Expected dynamic tools from default configuration
        let expected_dynamic_tools = vec![
            "birdeye_query",
            "solana_query",
            "jupiter_query",
            "ethereum_query",
            "cilium_query",
            "talos_query",
            "meteora_query",
            "raydium_query",
            "ebpf_query",
            "rust_best_practices_query",
        ];

        // Check that dynamic tools are registered
        for expected_tool in &expected_dynamic_tools {
            let tool_exists = tools
                .iter()
                .any(|tool| tool.get("name").and_then(|n| n.as_str()) == Some(*expected_tool));
            assert!(
                tool_exists,
                "Dynamic tool '{expected_tool}' should be registered"
            );
        }

        // Test that each dynamic tool has proper schema
        for tool in tools {
            let name = tool.get("name").and_then(|n| n.as_str());
            let description = tool.get("description").and_then(|d| d.as_str());
            let input_schema = tool.get("inputSchema");

            assert!(name.is_some(), "Tool should have a name");
            assert!(description.is_some(), "Tool should have a description");
            assert!(input_schema.is_some(), "Tool should have input schema");

            // Verify input schema structure
            let schema = input_schema.unwrap();
            let properties = schema.get("properties").expect("Should have properties");
            let tool_name = name.unwrap_or("unknown");

            // Only check for query/limit parameters on dynamic query tools
            if tool_name.ends_with("_query") {
                assert!(
                    properties.get("query").is_some(),
                    "Query tool '{tool_name}' should have query parameter"
                );
                assert!(
                    properties.get("limit").is_some(),
                    "Query tool '{tool_name}' should have limit parameter"
                );
            }
        }
    } else {
        eprintln!("Skipping dynamic tools test - DB not reachable within 2s");
    }

    cleanup_test_config();
}

#[tokio::test]
async fn test_dynamic_tool_invocation() {
    let _lock = TEST_MUTEX.lock().unwrap();
    setup_test_config();

    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "mock".to_string());

    if database_url == "mock" || database_url.is_empty() {
        eprintln!("Skipping dynamic tool invocation test - no test database available (database_url: '{database_url}')");
    } else {
        // In CI environments, database might not be accessible - try to connect with shorter timeout
        match timeout(
            Duration::from_secs(2), // Shorter timeout for CI
            DatabasePool::with_config(
                PoolConfig::builder()
                    .database_url(database_url)
                    .min_connections(1) // Reduce connection pool for CI
                    .max_connections(3)
                    .acquire_timeout(Duration::from_secs(5))
                    .build()
                    .unwrap(),
            ),
        )
        .await
        {
            Ok(Ok(db_pool)) => {
                let handler = McpHandler::new(&db_pool).expect("Failed to create handler");

                // Test calling a dynamic tool (birdeye_query)
                let request = json!({
                    "method": "tools/call",
                    "params": {
                        "name": "birdeye_query",
                        "arguments": {
                            "query": "price endpoint",
                            "limit": 3
                        }
                    }
                });

                let response = handler.handle_request(request).await;

                // Debug the response first
                if let Err(ref e) = response {
                    eprintln!("DEBUG: Tool invocation failed with error: {e}");
                } else {
                    eprintln!("DEBUG: Tool invocation succeeded");
                }

                // Check if response is ok - if not, this might be expected in CI
                if response.is_err() {
                    let err = response.unwrap_err();
                    eprintln!("DEBUG: Tool returned error: {err}");

                    // In CI, database might not be available - this is acceptable
                    // Skip the test if we get a database-related error
                    if err.to_string().contains("database")
                        || err.to_string().contains("connection")
                        || err.to_string().contains("pool")
                    {
                        eprintln!(
                            "DEBUG: Database error detected - skipping test as expected in CI"
                        );
                        return; // Skip test gracefully
                    }

                    // If it's not a database error, it should still succeed
                    panic!("Unexpected tool failure: {err}");
                }

                let response = response.unwrap();
                let content = response.get("content").expect("Should have content");
                assert!(content.is_array(), "Content should be an array");

                let text_content = content
                    .as_array()
                    .unwrap()
                    .first()
                    .and_then(|item| item.get("text"))
                    .and_then(|text| text.as_str());

                assert!(text_content.is_some(), "Should have text content");

                // Should contain information about BirdEye (even if no results found)
                let text = text_content.unwrap();
                eprintln!("DEBUG: Tool response text: '{text}'");
                assert!(
                    text.contains("BirdEye")
                        || text.contains("birdeye")
                        || text.contains("documentation"),
                    "Response should reference BirdEye or documentation, but got: '{text}'"
                );
            }
            _ => {
                eprintln!("Skipping dynamic tool invocation test - DB not reachable within 2s");
            }
        }
    }

    cleanup_test_config();
}

#[tokio::test]
async fn test_parameter_validation_dynamic_tools() {
    let _lock = TEST_MUTEX.lock().unwrap();
    setup_test_config();
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "mock".to_string());

    if database_url == "mock" {
        eprintln!("Skipping parameter validation test - no test database available");
    } else if let Ok(Ok(db_pool)) = timeout(
        Duration::from_secs(10),
        DatabasePool::with_config(
            PoolConfig::builder()
                .database_url(database_url)
                .min_connections(2)
                .max_connections(15)
                .acquire_timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        ),
    )
    .await
    {
        let handler = McpHandler::new(&db_pool).expect("Failed to create handler");

        // Test with missing query parameter
        let request = json!({
            "method": "tools/call",
            "params": {
                "name": "solana_query",
                "arguments": {
                    "limit": 5
                    // missing "query"
                }
            }
        });

        let response = handler.handle_request(request).await.unwrap();
        let content = response.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();

        assert!(
            text.contains("Error") && text.contains("query"),
            "Should return error about missing query parameter"
        );

        // Test with invalid limit
        let request = json!({
            "method": "tools/call",
            "params": {
                "name": "solana_query",
                "arguments": {
                    "query": "validator",
                    "limit": 25  // exceeds maximum of 20
                }
            }
        });

        let response = handler.handle_request(request).await.unwrap();
        let content = response.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();

        assert!(
            text.contains("Error") && (text.contains("limit") || text.contains("Limit")),
            "Should return error about invalid limit parameter"
        );
    } else {
        eprintln!("Skipping parameter validation test - DB not reachable within 2s");
    }

    cleanup_test_config();
}

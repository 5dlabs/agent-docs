//! Integration tests for SSE functionality

use doc_server_mcp::sse::{SSEConfig, ConnectionManager, HeartbeatService};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

/// Test SSE configuration loading
#[tokio::test]
async fn test_sse_config_integration() {
    // Set environment variables
    std::env::set_var("SSE_HEARTBEAT_INTERVAL", "10");
    std::env::set_var("SSE_CONNECTION_TIMEOUT", "30");
    std::env::set_var("SSE_BUFFER_SIZE", "100");
    
    let config = SSEConfig::from_env();
    
    assert_eq!(config.heartbeat_interval, Duration::from_secs(10));
    assert_eq!(config.connection_timeout, Duration::from_secs(30));
    assert_eq!(config.message_buffer_size, 100);
    
    // Cleanup
    std::env::remove_var("SSE_HEARTBEAT_INTERVAL");
    std::env::remove_var("SSE_CONNECTION_TIMEOUT");
    std::env::remove_var("SSE_BUFFER_SIZE");
}

/// Test connection manager with multiple connections
#[tokio::test]
async fn test_connection_manager_multiple_connections() {
    let config = SSEConfig::default();
    let manager = ConnectionManager::new(config);
    
    // Add multiple connections
    let mut connection_ids = Vec::new();
    for _ in 0..5 {
        let connection = manager.add_connection().await;
        connection_ids.push(connection.id);
    }
    
    // Verify all connections exist
    let connections = manager.get_all_connections().await;
    assert_eq!(connections.len(), 5);
    
    for id in &connection_ids {
        assert!(manager.get_connection(*id).await.is_some());
    }
    
    // Remove connections one by one
    for id in &connection_ids {
        assert!(manager.remove_connection(*id).await);
    }
    
    // Verify all connections removed
    let connections = manager.get_all_connections().await;
    assert!(connections.is_empty());
}

/// Test connection timeout and cleanup
#[tokio::test]
async fn test_connection_timeout_cleanup() {
    let mut config = SSEConfig::default();
    config.connection_timeout = Duration::from_millis(50);
    
    let manager = ConnectionManager::new(config);
    
    // Add connections
    let conn1 = manager.add_connection().await;
    let conn2 = manager.add_connection().await;
    let conn3 = manager.add_connection().await;
    
    // Update activity for one connection to keep it alive
    tokio::spawn(async move {
        loop {
            conn2.update_activity().await;
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Wait for timeout
    sleep(Duration::from_millis(100)).await;
    
    // Cleanup timed out connections
    let removed_count = manager.cleanup_timed_out_connections().await;
    
    // Should remove 2 connections (conn1 and conn3), conn2 should remain active
    assert_eq!(removed_count, 2);
    
    let active_connections = manager.get_all_connections().await;
    assert_eq!(active_connections.len(), 1);
}

/// Test heartbeat service broadcast
#[tokio::test]
async fn test_heartbeat_service_integration() {
    let config = SSEConfig::default();
    let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
    let heartbeat_service = HeartbeatService::new(connection_manager.clone(), config);
    
    // Add connections
    let conn1 = connection_manager.add_connection().await;
    let conn2 = connection_manager.add_connection().await;
    
    // Test immediate heartbeat
    let sent_count = heartbeat_service.send_immediate_heartbeat().await;
    
    // Since connections don't have active senders, messages should be buffered
    assert_eq!(sent_count, 0);
    
    // Verify messages were buffered
    {
        let buffer1 = conn1.message_buffer.read().await;
        let buffer2 = conn2.message_buffer.read().await;
        
        assert_eq!(buffer1.message_count(), 1);
        assert_eq!(buffer2.message_count(), 1);
    }
    
    // Test custom message broadcast
    let custom_message = json!({
        "type": "custom",
        "data": "test message"
    });
    
    let sent_count = heartbeat_service.broadcast_message(custom_message).await;
    assert_eq!(sent_count, 0); // Again, buffered since no active senders
    
    // Verify custom messages were buffered
    {
        let buffer1 = conn1.message_buffer.read().await;
        let buffer2 = conn2.message_buffer.read().await;
        
        assert_eq!(buffer1.message_count(), 2); // heartbeat + custom message
        assert_eq!(buffer2.message_count(), 2); // heartbeat + custom message
    }
}

/// Test message buffering and replay functionality
#[tokio::test]
async fn test_message_buffer_replay() {
    let config = SSEConfig::default();
    let connection_manager = ConnectionManager::new(config);
    let connection = connection_manager.add_connection().await;
    
    // Add messages to buffer
    let messages = vec![
        json!({"type": "message", "id": 1}),
        json!({"type": "message", "id": 2}),
        json!({"type": "message", "id": 3}),
    ];
    
    for message in &messages {
        let _ = connection.send_message(message.clone()).await;
    }
    
    // Verify messages were buffered
    {
        let buffer = connection.message_buffer.read().await;
        assert_eq!(buffer.message_count(), 3);
    }
    
    // Drain messages (simulate replay)
    let buffered_messages = {
        let mut buffer = connection.message_buffer.write().await;
        buffer.drain_messages().await
    };
    
    assert_eq!(buffered_messages.len(), 3);
    
    // Verify buffer is empty after drain
    {
        let buffer = connection.message_buffer.read().await;
        assert!(buffer.is_empty());
    }
}

/// Test connection statistics
#[tokio::test]
async fn test_connection_statistics() {
    let config = SSEConfig::default();
    let manager = ConnectionManager::new(config);
    
    // Add connections
    let _conn1 = manager.add_connection().await;
    let _conn2 = manager.add_connection().await;
    let _conn3 = manager.add_connection().await;
    
    // Get statistics
    let stats = manager.get_stats().await;
    
    assert_eq!(stats["active_connections"], 3);
    assert!(stats["average_duration_seconds"].is_u64());
    assert_eq!(stats["total_buffered_messages"], 0);
    assert!(stats["cleanup_interval_seconds"].is_u64());
    assert!(stats["heartbeat_interval_seconds"].is_u64());
}

/// Test concurrent connection operations
#[tokio::test]
async fn test_concurrent_connection_operations() {
    let config = SSEConfig::default();
    let manager = Arc::new(ConnectionManager::new(config));
    
    // Spawn multiple tasks that add connections concurrently
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let connection = manager_clone.add_connection().await;
            
            // Simulate some activity
            for j in 0..5 {
                connection.update_activity().await;
                let message = json!({"task": i, "message": j});
                connection.send_message(message).await;
                sleep(Duration::from_millis(1)).await;
            }
            
            connection.id
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let mut connection_ids = Vec::new();
    for handle in handles {
        let connection_id = handle.await.unwrap();
        connection_ids.push(connection_id);
    }
    
    // Verify all connections were added
    let connections = manager.get_all_connections().await;
    assert_eq!(connections.len(), 10);
    
    // Verify each connection has buffered messages
    for connection in &connections {
        let buffer = connection.message_buffer.read().await;
        assert_eq!(buffer.message_count(), 5);
    }
    
    // Clean up all connections
    for id in connection_ids {
        assert!(manager.remove_connection(id).await);
    }
    
    assert!(manager.get_all_connections().await.is_empty());
}

/// Test buffer overflow handling under load
#[tokio::test]
async fn test_buffer_overflow_under_load() {
    let mut config = SSEConfig::default();
    config.message_buffer_size = 10; // Small buffer to force overflow
    
    let connection_manager = ConnectionManager::new(config);
    let connection = connection_manager.add_connection().await;
    
    // Send more messages than buffer can hold
    for i in 0..20 {
        let message = json!({"id": i, "data": format!("Message {}", i)});
        connection.send_message(message).await;
    }
    
    // Buffer should only contain the last 10 messages
    let buffer = connection.message_buffer.read().await;
    assert_eq!(buffer.message_count(), 10);
    
    // Verify utilization is at 100%
    assert_eq!(buffer.utilization_percentage(), 100.0);
}

/// Test heartbeat service startup and basic operation
#[tokio::test]
async fn test_heartbeat_service_startup() {
    let mut config = SSEConfig::default();
    config.heartbeat_interval = Duration::from_millis(50); // Fast heartbeat for testing
    
    let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
    let heartbeat_service = Arc::new(HeartbeatService::new(connection_manager.clone(), config));
    
    // Add a connection
    let connection = connection_manager.add_connection().await;
    
    // Start heartbeat service
    let heartbeat_handle = heartbeat_service.start();
    
    // Wait for a few heartbeat cycles
    sleep(Duration::from_millis(150)).await;
    
    // Check if messages were buffered (since no active sender)
    let buffer = connection.message_buffer.read().await;
    assert!(buffer.message_count() >= 2); // Should have received at least 2 heartbeats
    
    // Stop the heartbeat service
    heartbeat_handle.abort();
}

/// Test cleanup task operation
#[tokio::test] 
async fn test_cleanup_task_operation() {
    let mut config = SSEConfig::default();
    config.connection_timeout = Duration::from_millis(30); // Short timeout for testing
    
    let manager = Arc::new(ConnectionManager::new(config));
    
    // Start cleanup task
    let cleanup_handle = manager.start_cleanup_task();
    
    // Add connections
    let _conn1 = manager.add_connection().await;
    let _conn2 = manager.add_connection().await;
    
    assert_eq!(manager.get_all_connections().await.len(), 2);
    
    // Wait for connections to timeout and be cleaned up
    sleep(Duration::from_millis(100)).await;
    
    // Connections should be cleaned up automatically
    let connections = manager.get_all_connections().await;
    assert_eq!(connections.len(), 0);
    
    // Stop cleanup task
    cleanup_handle.abort();
}
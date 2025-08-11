//! Unit tests for SSE module

use super::*;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

mod config_tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = SSEConfig::default();
        
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.connection_timeout, Duration::from_secs(90));
        assert_eq!(config.initial_retry_delay, Duration::from_secs(1));
        assert_eq!(config.max_retry_delay, Duration::from_secs(60));
        assert_eq!(config.retry_jitter_max, Duration::from_millis(500));
        assert_eq!(config.message_buffer_size, 1000);
        assert_eq!(config.buffer_retention, Duration::from_secs(300));
        assert!(!config.enable_redis_persistence);
    }
    
    #[test]
    fn test_config_from_env() {
        std::env::set_var("SSE_HEARTBEAT_INTERVAL", "15");
        std::env::set_var("SSE_CONNECTION_TIMEOUT", "120");
        std::env::set_var("SSE_BUFFER_SIZE", "500");
        std::env::set_var("SSE_ENABLE_REDIS", "true");
        
        let config = SSEConfig::from_env();
        
        assert_eq!(config.heartbeat_interval, Duration::from_secs(15));
        assert_eq!(config.connection_timeout, Duration::from_secs(120));
        assert_eq!(config.message_buffer_size, 500);
        assert!(config.enable_redis_persistence);
        
        // Cleanup
        std::env::remove_var("SSE_HEARTBEAT_INTERVAL");
        std::env::remove_var("SSE_CONNECTION_TIMEOUT");
        std::env::remove_var("SSE_BUFFER_SIZE");
        std::env::remove_var("SSE_ENABLE_REDIS");
    }
}

mod connection_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_creation() {
        let config = SSEConfig::default();
        let connection = Connection::new(&config);
        
        assert!(!connection.id.is_nil());
        assert!(connection.duration() < Duration::from_millis(100)); // Just created
    }
    
    #[tokio::test]
    async fn test_connection_activity_update() {
        let config = SSEConfig::default();
        let connection = Connection::new(&config);
        
        sleep(Duration::from_millis(10)).await;
        let initial_activity = {
            let last_activity = connection.last_activity.read().await;
            *last_activity
        };
        
        sleep(Duration::from_millis(10)).await;
        connection.update_activity().await;
        
        let updated_activity = {
            let last_activity = connection.last_activity.read().await;
            *last_activity
        };
        
        assert!(updated_activity > initial_activity);
    }
    
    #[tokio::test]
    async fn test_connection_timeout_detection() {
        let mut config = SSEConfig::default();
        config.connection_timeout = Duration::from_millis(10);
        let connection = Connection::new(&config);
        
        // Should not be timed out initially
        assert!(!connection.is_timed_out(config.connection_timeout).await);
        
        // Wait for timeout
        sleep(Duration::from_millis(20)).await;
        
        // Should be timed out now
        assert!(connection.is_timed_out(config.connection_timeout).await);
    }
    
    #[tokio::test]
    async fn test_connection_info() {
        let config = SSEConfig::default();
        let connection = Connection::new(&config);
        
        let info = connection.get_info().await;
        
        assert!(info["id"].is_string());
        assert!(info["created_at"].is_u64());
        assert!(info["last_activity"].is_u64());
        assert_eq!(info["buffered_messages"], 0);
    }
}

mod connection_manager_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_manager_creation() {
        let config = SSEConfig::default();
        let manager = ConnectionManager::new(config);
        
        let connections = manager.get_all_connections().await;
        assert!(connections.is_empty());
    }
    
    #[tokio::test]
    async fn test_add_and_remove_connection() {
        let config = SSEConfig::default();
        let manager = ConnectionManager::new(config);
        
        // Add connection
        let connection = manager.add_connection().await;
        let connection_id = connection.id;
        
        assert_eq!(manager.get_all_connections().await.len(), 1);
        assert!(manager.get_connection(connection_id).await.is_some());
        
        // Remove connection
        let removed = manager.remove_connection(connection_id).await;
        assert!(removed);
        assert!(manager.get_connection(connection_id).await.is_none());
        assert_eq!(manager.get_all_connections().await.len(), 0);
    }
    
    #[tokio::test]
    async fn test_cleanup_timed_out_connections() {
        let mut config = SSEConfig::default();
        config.connection_timeout = Duration::from_millis(10);
        let manager = ConnectionManager::new(config.clone());
        
        // Add connections
        let _conn1 = manager.add_connection().await;
        let _conn2 = manager.add_connection().await;
        
        assert_eq!(manager.get_all_connections().await.len(), 2);
        
        // Wait for timeout
        sleep(Duration::from_millis(20)).await;
        
        // Cleanup should remove all connections
        let removed = manager.cleanup_timed_out_connections().await;
        assert_eq!(removed, 2);
        assert_eq!(manager.get_all_connections().await.len(), 0);
    }
    
    #[tokio::test]
    async fn test_get_stats() {
        let config = SSEConfig::default();
        let manager = ConnectionManager::new(config);
        
        let _conn1 = manager.add_connection().await;
        let _conn2 = manager.add_connection().await;
        
        let stats = manager.get_stats().await;
        
        assert_eq!(stats["active_connections"], 2);
        assert!(stats["average_duration_seconds"].is_u64());
        assert_eq!(stats["total_buffered_messages"], 0);
    }
    
    #[tokio::test]
    async fn test_broadcast_message() {
        let config = SSEConfig::default();
        let manager = ConnectionManager::new(config);
        
        let _conn1 = manager.add_connection().await;
        let _conn2 = manager.add_connection().await;
        
        let message = json!({"type": "test", "data": "hello"});
        let sent_count = manager.broadcast_message(message).await;
        
        // Since connections don't have senders set, messages should be buffered
        // and sent_count should be 0
        assert_eq!(sent_count, 0);
    }
}

mod message_buffer_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_buffer_basic_operations() {
        let config = SSEConfig::default();
        let mut buffer = MessageBuffer::new(&config);
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.message_count(), 0);
        
        buffer.add_message(json!({"test": "message1"})).await;
        assert_eq!(buffer.message_count(), 1);
        assert!(!buffer.is_empty());
        
        let messages = buffer.drain_messages().await;
        assert_eq!(messages.len(), 1);
        assert!(buffer.is_empty());
    }
    
    #[tokio::test]
    async fn test_buffer_overflow_handling() {
        let mut config = SSEConfig::default();
        config.message_buffer_size = 2;
        let mut buffer = MessageBuffer::new(&config);
        
        buffer.add_message(json!({"msg": 1})).await;
        buffer.add_message(json!({"msg": 2})).await;
        buffer.add_message(json!({"msg": 3})).await; // Should remove first message
        
        assert_eq!(buffer.message_count(), 2);
        
        let messages = buffer.drain_messages().await;
        assert_eq!(messages.len(), 2);
        // Should have messages 2 and 3, message 1 removed due to overflow
    }
    
    #[tokio::test]
    async fn test_buffer_utilization() {
        let mut config = SSEConfig::default();
        config.message_buffer_size = 10;
        let mut buffer = MessageBuffer::new(&config);
        
        for i in 0..5 {
            buffer.add_message(json!({"msg": i})).await;
        }
        
        assert_eq!(buffer.utilization_percentage(), 50.0);
    }
    
    #[tokio::test]
    async fn test_buffer_stats() {
        let config = SSEConfig::default();
        let mut buffer = MessageBuffer::new(&config);
        
        buffer.add_message(json!({"test": "message"})).await;
        
        let stats = buffer.get_stats();
        assert_eq!(stats["total_messages"], 1);
        assert_eq!(stats["max_capacity"], 1000);
        assert!(stats["utilization_percentage"].as_f64().unwrap() > 0.0);
    }
    
    #[tokio::test]
    async fn test_clear_buffer() {
        let config = SSEConfig::default();
        let mut buffer = MessageBuffer::new(&config);
        
        buffer.add_message(json!({"test": "message1"})).await;
        buffer.add_message(json!({"test": "message2"})).await;
        
        assert_eq!(buffer.message_count(), 2);
        
        buffer.clear().await;
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.message_count(), 0);
    }
}

mod heartbeat_tests {
    use super::*;
    use crate::sse::heartbeat::*;
    
    #[test]
    fn test_create_heartbeat_message() {
        let message = create_heartbeat_message(42);
        
        assert_eq!(message["type"], "heartbeat");
        assert_eq!(message["sequence"], 42);
        assert_eq!(message["server"], "doc-server-mcp");
        assert!(message["timestamp"].as_u64().is_some());
    }
    
    #[test]
    fn test_create_connection_message() {
        let connection_id = Uuid::new_v4();
        let message = create_connection_message(connection_id);
        
        assert_eq!(message["type"], "connected");
        assert_eq!(message["connection_id"], connection_id.to_string());
        assert_eq!(message["server"], "doc-server-mcp");
        assert!(message["timestamp"].as_u64().is_some());
    }
    
    #[test]
    fn test_create_connection_lost_message() {
        let message = create_connection_lost_message();
        
        assert_eq!(message["type"], "connection_lost");
        assert_eq!(message["server"], "doc-server-mcp");
        assert!(message["timestamp"].as_u64().is_some());
    }
    
    #[test]
    fn test_create_reconnection_message() {
        let connection_id = Uuid::new_v4();
        let message = create_reconnection_message(connection_id, 5);
        
        assert_eq!(message["type"], "reconnecting");
        assert_eq!(message["connection_id"], connection_id.to_string());
        assert_eq!(message["attempt"], 5);
        assert_eq!(message["server"], "doc-server-mcp");
        assert!(message["timestamp"].as_u64().is_some());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_heartbeat_service_broadcast() {
        let config = SSEConfig::default();
        let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
        let heartbeat_service = HeartbeatService::new(connection_manager.clone(), config);
        
        // Add some connections
        let _conn1 = connection_manager.add_connection().await;
        let _conn2 = connection_manager.add_connection().await;
        
        // Send immediate heartbeat
        let sent_count = heartbeat_service.send_immediate_heartbeat().await;
        
        // Since connections don't have senders, messages should be buffered
        assert_eq!(sent_count, 0);
    }
    
    #[tokio::test]
    async fn test_connection_lifecycle() {
        let config = SSEConfig::default();
        let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
        
        // Add connection
        let connection = connection_manager.add_connection().await;
        let connection_id = connection.id;
        
        // Verify connection exists
        assert!(connection_manager.get_connection(connection_id).await.is_some());
        
        // Update activity
        connection.update_activity().await;
        
        // Get connection info
        let info = connection.get_info().await;
        assert_eq!(info["id"], connection_id.to_string());
        
        // Remove connection
        assert!(connection_manager.remove_connection(connection_id).await);
        assert!(connection_manager.get_connection(connection_id).await.is_none());
    }
}
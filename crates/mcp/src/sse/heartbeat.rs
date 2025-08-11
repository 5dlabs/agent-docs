//! Heartbeat service for SSE connections

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use serde_json::{json, Value};
use tracing::{debug, info};

use crate::sse::{ConnectionManager, SSEConfig};

/// Heartbeat service that sends keep-alive messages
pub struct HeartbeatService {
    connection_manager: Arc<ConnectionManager>,
    config: SSEConfig,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(connection_manager: Arc<ConnectionManager>, config: SSEConfig) -> Self {
        Self {
            connection_manager,
            config,
        }
    }
    
    /// Start the heartbeat service
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let manager = self.connection_manager.clone();
        let heartbeat_interval = self.config.heartbeat_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);
            let mut sequence: u64 = 0;
            
            info!("Heartbeat service started with interval: {:?}", heartbeat_interval);
            
            loop {
                interval.tick().await;
                sequence += 1;
                
                let heartbeat_message = create_heartbeat_message(sequence);
                let sent_count = manager.broadcast_message(heartbeat_message).await;
                
                debug!("Heartbeat #{} sent to {} connections", sequence, sent_count);
                
                // Log statistics periodically (every 10 heartbeats)
                if sequence % 10 == 0 {
                    let stats = manager.get_stats().await;
                    info!("SSE statistics: {}", stats);
                }
            }
        })
    }
    
    /// Send immediate heartbeat to all connections
    pub async fn send_immediate_heartbeat(&self) -> usize {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let message = json!({
            "type": "heartbeat",
            "timestamp": timestamp,
            "immediate": true
        });
        
        let sent_count = self.connection_manager.broadcast_message(message).await;
        info!("Immediate heartbeat sent to {} connections", sent_count);
        sent_count
    }
    
    /// Send a custom message to all connections
    pub async fn broadcast_message(&self, message: Value) -> usize {
        self.connection_manager.broadcast_message(message).await
    }
}

/// Create a heartbeat message with timestamp and sequence
fn create_heartbeat_message(sequence: u64) -> Value {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    json!({
        "type": "heartbeat",
        "timestamp": timestamp,
        "sequence": sequence,
        "server": "doc-server-mcp"
    })
}

/// Create a connection established message
pub fn create_connection_message(connection_id: uuid::Uuid) -> Value {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    json!({
        "type": "connected",
        "connection_id": connection_id,
        "timestamp": timestamp,
        "server": "doc-server-mcp",
        "message": "SSE connection established successfully"
    })
}

/// Create a connection lost message
pub fn create_connection_lost_message() -> Value {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    json!({
        "type": "connection_lost",
        "timestamp": timestamp,
        "server": "doc-server-mcp",
        "message": "SSE connection lost, attempting to reconnect"
    })
}

/// Create a reconnection message
pub fn create_reconnection_message(connection_id: uuid::Uuid, attempt: u32) -> Value {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    json!({
        "type": "reconnecting",
        "connection_id": connection_id,
        "timestamp": timestamp,
        "attempt": attempt,
        "server": "doc-server-mcp",
        "message": format!("Attempting to reconnect (attempt #{})", attempt)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let connection_id = uuid::Uuid::new_v4();
        let message = create_connection_message(connection_id);
        
        assert_eq!(message["type"], "connected");
        assert_eq!(message["connection_id"], connection_id.to_string());
        assert_eq!(message["server"], "doc-server-mcp");
        assert!(message["timestamp"].as_u64().is_some());
    }
}
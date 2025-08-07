//! Connection management for SSE

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};
use tokio::time::interval;
use uuid::Uuid;
use tracing::{debug, info, warn};
use serde_json::{json, Value};

use crate::sse::{MessageBuffer, SSEConfig};

/// Represents a single SSE connection
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: Uuid,
    pub created_at: Instant,
    pub last_activity: Arc<RwLock<Instant>>,
    pub message_buffer: Arc<RwLock<MessageBuffer>>,
    pub sender: Arc<RwLock<Option<mpsc::UnboundedSender<Value>>>>,
    pub client_info: Arc<RwLock<HashMap<String, Value>>>,
}

impl Connection {
    /// Create a new connection
    pub fn new(config: &SSEConfig) -> Self {
        let id = Uuid::new_v4();
        let now = Instant::now();
        
        Self {
            id,
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
            message_buffer: Arc::new(RwLock::new(MessageBuffer::new(config))),
            sender: Arc::new(RwLock::new(None)),
            client_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update last activity timestamp
    pub async fn update_activity(&self) {
        let mut last_activity = self.last_activity.write().await;
        *last_activity = Instant::now();
    }
    
    /// Check if connection has timed out
    pub async fn is_timed_out(&self, timeout: Duration) -> bool {
        let last_activity = self.last_activity.read().await;
        last_activity.elapsed() > timeout
    }
    
    /// Send a message to the connection
    pub async fn send_message(&self, message: Value) -> bool {
        let sender = self.sender.read().await;
        if let Some(sender) = sender.as_ref() {
            if sender.send(message.clone()).is_ok() {
                self.update_activity().await;
                return true;
            }
        }
        
        // If direct send failed, buffer the message
        let mut buffer = self.message_buffer.write().await;
        buffer.add_message(message).await;
        false
    }
    
    /// Set the message sender channel
    pub async fn set_sender(&self, sender: mpsc::UnboundedSender<Value>) {
        let mut sender_guard = self.sender.write().await;
        *sender_guard = Some(sender);
    }
    
    /// Remove the message sender channel
    pub async fn remove_sender(&self) {
        let mut sender_guard = self.sender.write().await;
        *sender_guard = None;
    }
    
    /// Get connection duration
    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    /// Get connection info for monitoring
    pub async fn get_info(&self) -> Value {
        let last_activity = self.last_activity.read().await;
        let buffer = self.message_buffer.read().await;
        let client_info = self.client_info.read().await;
        
        json!({
            "id": self.id,
            "created_at": self.created_at.elapsed().as_secs(),
            "last_activity": last_activity.elapsed().as_secs(),
            "buffered_messages": buffer.message_count(),
            "client_info": *client_info
        })
    }
}

/// Manages all SSE connections
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<Uuid, Arc<Connection>>>>,
    config: SSEConfig,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config: SSEConfig) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Add a new connection
    pub async fn add_connection(&self) -> Arc<Connection> {
        let connection = Arc::new(Connection::new(&self.config));
        let mut connections = self.connections.write().await;
        connections.insert(connection.id, connection.clone());
        
        info!("SSE connection added: {}", connection.id);
        connection
    }
    
    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: Uuid) -> bool {
        let mut connections = self.connections.write().await;
        let removed = connections.remove(&connection_id).is_some();
        
        if removed {
            info!("SSE connection removed: {}", connection_id);
        }
        
        removed
    }
    
    /// Get a connection by ID
    pub async fn get_connection(&self, connection_id: Uuid) -> Option<Arc<Connection>> {
        let connections = self.connections.read().await;
        connections.get(&connection_id).cloned()
    }
    
    /// Get all active connections
    pub async fn get_all_connections(&self) -> Vec<Arc<Connection>> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }
    
    /// Clean up timed out connections
    pub async fn cleanup_timed_out_connections(&self) -> usize {
        let mut connections = self.connections.write().await;
        let mut to_remove = Vec::new();
        
        for (id, connection) in connections.iter() {
            if connection.is_timed_out(self.config.connection_timeout).await {
                to_remove.push(*id);
            }
        }
        
        let removed_count = to_remove.len();
        for id in to_remove {
            connections.remove(&id);
            warn!("Removed timed out SSE connection: {}", id);
        }
        
        if removed_count > 0 {
            info!("Cleaned up {} timed out SSE connections", removed_count);
        }
        
        removed_count
    }
    
    /// Get connection statistics
    pub async fn get_stats(&self) -> Value {
        let connections = self.connections.read().await;
        let active_count = connections.len();
        
        let mut total_duration = Duration::new(0, 0);
        let mut total_buffered_messages = 0;
        
        for connection in connections.values() {
            total_duration += connection.duration();
            let buffer = connection.message_buffer.read().await;
            total_buffered_messages += buffer.message_count();
        }
        
        let average_duration_secs = if active_count > 0 {
            total_duration.as_secs() / active_count as u64
        } else {
            0
        };
        
        json!({
            "active_connections": active_count,
            "average_duration_seconds": average_duration_secs,
            "total_buffered_messages": total_buffered_messages,
            "cleanup_interval_seconds": self.config.connection_timeout.as_secs(),
            "heartbeat_interval_seconds": self.config.heartbeat_interval.as_secs()
        })
    }
    
    /// Start the cleanup task
    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let manager = Arc::new(self.clone());
        let cleanup_interval = self.config.connection_timeout / 3; // Check every 1/3 of timeout period
        
        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                match manager.cleanup_timed_out_connections().await {
                    0 => debug!("SSE cleanup: no connections to remove"),
                    count => info!("SSE cleanup: removed {} timed out connections", count),
                }
            }
        })
    }
    
    /// Broadcast message to all connections
    pub async fn broadcast_message(&self, message: Value) -> usize {
        let connections = self.connections.read().await;
        let mut sent_count = 0;
        
        for connection in connections.values() {
            if connection.send_message(message.clone()).await {
                sent_count += 1;
            }
        }
        
        if sent_count > 0 {
            debug!("Broadcast message sent to {} connections", sent_count);
        }
        
        sent_count
    }
}

impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            config: self.config.clone(),
        }
    }
}
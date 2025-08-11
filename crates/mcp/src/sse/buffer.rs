//! Message buffering for SSE connections

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::sse::SSEConfig;

/// Represents a buffered message
#[derive(Debug, Clone)]
pub struct BufferedMessage {
    pub id: Uuid,
    pub content: Value,
    pub timestamp: Instant,
    pub retry_count: u32,
}

impl BufferedMessage {
    /// Create a new buffered message
    pub fn new(content: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            timestamp: Instant::now(),
            retry_count: 0,
        }
    }
    
    /// Check if message has expired
    pub fn is_expired(&self, retention: Duration) -> bool {
        self.timestamp.elapsed() > retention
    }
    
    /// Get message age
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Message buffer for a single connection
#[derive(Debug)]
pub struct MessageBuffer {
    buffer: VecDeque<BufferedMessage>,
    max_size: usize,
    retention: Duration,
}

impl MessageBuffer {
    /// Create a new message buffer
    pub fn new(config: &SSEConfig) -> Self {
        Self {
            buffer: VecDeque::new(),
            max_size: config.message_buffer_size,
            retention: config.buffer_retention,
        }
    }
    
    /// Add a message to the buffer
    pub async fn add_message(&mut self, content: Value) {
        let message = BufferedMessage::new(content);
        
        // Remove expired messages first
        self.cleanup_expired_messages();
        
        // If buffer is full, remove oldest messages (FIFO)
        while self.buffer.len() >= self.max_size {
            if let Some(removed) = self.buffer.pop_front() {
                warn!("Buffer overflow: removed message {}", removed.id);
            }
        }
        
        self.buffer.push_back(message);
        debug!("Message buffered. Buffer size: {}", self.buffer.len());
    }
    
    /// Get all buffered messages and clear the buffer
    pub async fn drain_messages(&mut self) -> Vec<BufferedMessage> {
        self.cleanup_expired_messages();
        let messages: Vec<BufferedMessage> = self.buffer.drain(..).collect();
        
        if !messages.is_empty() {
            debug!("Drained {} messages from buffer", messages.len());
        }
        
        messages
    }
    
    /// Get buffered messages without removing them
    pub async fn peek_messages(&self) -> Vec<BufferedMessage> {
        self.buffer
            .iter()
            .filter(|msg| !msg.is_expired(self.retention))
            .cloned()
            .collect()
    }
    
    /// Get the number of messages in buffer
    pub fn message_count(&self) -> usize {
        self.buffer.len()
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    
    /// Get buffer utilization as percentage
    pub fn utilization_percentage(&self) -> f32 {
        (self.buffer.len() as f32 / self.max_size as f32) * 100.0
    }
    
    /// Remove expired messages from buffer
    fn cleanup_expired_messages(&mut self) {
        let initial_len = self.buffer.len();
        
        self.buffer.retain(|msg| !msg.is_expired(self.retention));
        
        let removed = initial_len - self.buffer.len();
        if removed > 0 {
            debug!("Removed {} expired messages from buffer", removed);
        }
    }
    
    /// Clear all messages from buffer
    pub async fn clear(&mut self) {
        let cleared = self.buffer.len();
        self.buffer.clear();
        
        if cleared > 0 {
            debug!("Cleared {} messages from buffer", cleared);
        }
    }
    
    /// Get buffer statistics
    pub fn get_stats(&self) -> Value {
        let mut expired_count = 0;
        let mut oldest_age_secs = 0u64;
        
        for message in &self.buffer {
            if message.is_expired(self.retention) {
                expired_count += 1;
            }
            let age_secs = message.age().as_secs();
            if age_secs > oldest_age_secs {
                oldest_age_secs = age_secs;
            }
        }
        
        json!({
            "total_messages": self.buffer.len(),
            "expired_messages": expired_count,
            "max_capacity": self.max_size,
            "utilization_percentage": self.utilization_percentage(),
            "oldest_message_age_seconds": oldest_age_secs,
            "retention_seconds": self.retention.as_secs()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

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
}
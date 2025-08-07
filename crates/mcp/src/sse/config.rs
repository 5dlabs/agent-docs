//! SSE configuration

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// SSE configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEConfig {
    /// Interval between heartbeat messages
    pub heartbeat_interval: Duration,
    
    /// Connection timeout after no activity
    pub connection_timeout: Duration,
    
    /// Initial retry delay for client reconnection
    pub initial_retry_delay: Duration,
    
    /// Maximum retry delay for client reconnection
    pub max_retry_delay: Duration,
    
    /// Maximum jitter to add to retry delays
    pub retry_jitter_max: Duration,
    
    /// Maximum number of messages to buffer per connection
    pub message_buffer_size: usize,
    
    /// How long to retain buffered messages
    pub buffer_retention: Duration,
    
    /// Whether to enable Redis persistence for message buffers
    pub enable_redis_persistence: bool,
}

impl Default for SSEConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(90),
            initial_retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(60),
            retry_jitter_max: Duration::from_millis(500),
            message_buffer_size: 1000,
            buffer_retention: Duration::from_secs(300), // 5 minutes
            enable_redis_persistence: false,
        }
    }
}

impl SSEConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(interval) = std::env::var("SSE_HEARTBEAT_INTERVAL") {
            if let Ok(secs) = interval.parse::<u64>() {
                config.heartbeat_interval = Duration::from_secs(secs);
            }
        }
        
        if let Ok(timeout) = std::env::var("SSE_CONNECTION_TIMEOUT") {
            if let Ok(secs) = timeout.parse::<u64>() {
                config.connection_timeout = Duration::from_secs(secs);
            }
        }
        
        if let Ok(buffer_size) = std::env::var("SSE_BUFFER_SIZE") {
            if let Ok(size) = buffer_size.parse::<usize>() {
                config.message_buffer_size = size;
            }
        }
        
        if let Ok(enable_redis) = std::env::var("SSE_ENABLE_REDIS") {
            config.enable_redis_persistence = enable_redis.to_lowercase() == "true";
        }
        
        config
    }
}
# Connection Reliability Design for Doc Server

## Problem Statement

The current implementation experiences timeout issues when connected through Toolman:
- SSE connections drop without proper recovery
- No keep-alive mechanism for long-lived connections
- Toolman integration requires robust connection handling

## Current State Analysis

### Existing Features
```rust
struct McpConnectionConfig {
    initialize_timeout: Duration,    // 30 seconds
    max_retries: u32,               // 3 (not implemented)
    retry_base_delay: Duration,     // 500ms (not implemented)
    retry_max_delay: Duration,      // 10s (not implemented)
}
```

### Issues
1. No SSE heartbeat/keep-alive implementation
2. Retry logic is defined but not implemented
3. No automatic reconnection on connection loss
4. No connection health monitoring

## Proposed Solution

### 1. SSE Keep-Alive Mechanism

```rust
// src/connection/keepalive.rs
pub struct SseKeepAlive {
    interval: Duration,
    timeout: Duration,
    last_activity: Arc<Mutex<Instant>>,
}

impl SseKeepAlive {
    pub fn new(interval: Duration, timeout: Duration) -> Self {
        Self {
            interval,      // Default: 30 seconds
            timeout,       // Default: 90 seconds
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    pub async fn start_heartbeat(&self, tx: mpsc::Sender<SseMessage>) {
        let mut interval = tokio::time::interval(self.interval);
        
        loop {
            interval.tick().await;
            
            // Send SSE comment as heartbeat (doesn't affect protocol)
            if tx.send(SseMessage::Comment("keepalive".to_string())).await.is_err() {
                break; // Connection closed
            }
            
            // Check for timeout
            let last = *self.last_activity.lock().unwrap();
            if last.elapsed() > self.timeout {
                warn!("Connection timeout detected, closing");
                break;
            }
        }
    }
    
    pub fn record_activity(&self) {
        *self.last_activity.lock().unwrap() = Instant::now();
    }
}
```

### 2. Enhanced Connection Handler

```rust
// src/bin/http_server.rs updates
async fn handle_mcp_connection_with_resilience(
    handler: McpHandler,
    transport: rmcp::transport::sse_server::SseServerTransport,
    config: McpConnectionConfig,
    connection_id: String,
) -> Result<(), ServerError> {
    // Create keep-alive manager
    let keepalive = SseKeepAlive::new(
        Duration::from_secs(30),  // Send heartbeat every 30s
        Duration::from_secs(90),  // Timeout after 90s of no activity
    );
    
    // Wrap transport with keep-alive monitoring
    let monitored_transport = MonitoredTransport::new(transport, keepalive.clone());
    
    // Start heartbeat task
    let heartbeat_handle = tokio::spawn({
        let ka = keepalive.clone();
        let tx = monitored_transport.sse_tx();
        async move {
            ka.start_heartbeat(tx).await;
        }
    });
    
    // Run service with monitoring
    let service_result = handler.serve(monitored_transport).await?;
    
    // Cleanup
    heartbeat_handle.abort();
    
    Ok(())
}
```

### 3. Client-Side Recovery (for Toolman)

```rust
// src/client/resilient_client.rs
pub struct ResilientMcpClient {
    base_url: String,
    reconnect_delay: Duration,
    max_reconnect_attempts: u32,
}

impl ResilientMcpClient {
    pub async fn connect_with_retry(&self) -> Result<McpConnection, Error> {
        let mut attempts = 0;
        let mut delay = self.reconnect_delay;
        
        loop {
            match self.try_connect().await {
                Ok(conn) => {
                    info!("Successfully connected to MCP server");
                    return Ok(conn);
                }
                Err(e) if attempts < self.max_reconnect_attempts => {
                    attempts += 1;
                    warn!("Connection attempt {} failed: {}, retrying in {:?}", 
                          attempts, e, delay);
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(60));
                }
                Err(e) => {
                    error!("Failed to connect after {} attempts", attempts);
                    return Err(e);
                }
            }
        }
    }
    
    async fn monitor_connection(&self, conn: McpConnection) {
        loop {
            if let Err(e) = conn.health_check().await {
                warn!("Health check failed: {}, reconnecting", e);
                if let Ok(new_conn) = self.connect_with_retry().await {
                    // Replace connection
                    conn.replace(new_conn);
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
```

### 4. SSE Protocol Extensions

```rust
// SSE message types for connection management
enum SseMessage {
    // Standard MCP messages
    Data(String),
    
    // Connection management
    Comment(String),      // : keepalive\n\n
    Retry(u64),          // retry: 5000\n\n
    Id(String),          // id: msg-123\n\n
}

// Enhanced SSE transport
struct EnhancedSseTransport {
    // Track last message ID for recovery
    last_message_id: Option<String>,
    
    // Buffer unacknowledged messages
    pending_messages: VecDeque<(String, String)>, // (id, data)
    
    // Connection state
    connected: AtomicBool,
}
```

### 5. Configuration Updates

```yaml
# Environment variables
CONNECTION_KEEPALIVE_INTERVAL: "30s"    # How often to send heartbeats
CONNECTION_KEEPALIVE_TIMEOUT: "90s"     # When to consider connection dead
CONNECTION_RETRY_ATTEMPTS: "5"          # Max reconnection attempts
CONNECTION_RETRY_DELAY: "1s"            # Initial retry delay
CONNECTION_RETRY_MAX_DELAY: "60s"       # Max retry delay
```

### 6. Health Endpoints Enhancement

```rust
// Enhanced health check that monitors SSE connections
async fn health_ready(State(app_state): State<AppState>) -> impl IntoResponse {
    let active_connections = app_state.connection_manager.active_count();
    let last_activity = app_state.connection_manager.last_activity();
    
    if active_connections == 0 && last_activity.elapsed() > Duration::from_secs(300) {
        // No connections for 5 minutes might indicate issues
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "degraded",
                "active_connections": 0,
                "last_activity_secs": last_activity.elapsed().as_secs()
            }))
        );
    }
    
    (StatusCode::OK, Json(json!({
        "status": "ready",
        "active_connections": active_connections,
        "uptime_secs": app_state.start_time.elapsed().as_secs()
    })))
}
```

## Implementation Plan

### Phase 1: Server-Side Keep-Alive (Week 1)
1. Implement `SseKeepAlive` struct
2. Add heartbeat sending to SSE transport
3. Monitor connection activity
4. Add connection timeout detection

### Phase 2: Connection Recovery (Week 2)
1. Implement retry logic in connection handler
2. Add message buffering for recovery
3. Track message IDs for resumption
4. Test with network interruptions

### Phase 3: Client Guidelines (Week 3)
1. Document Toolman integration requirements
2. Provide client-side recovery examples
3. Add connection monitoring endpoints
4. Create integration test suite

### Phase 4: Monitoring & Metrics (Week 4)
1. Add Prometheus metrics for connection health
2. Log connection lifecycle events
3. Track timeout and recovery statistics
4. Create alerting rules

## Testing Strategy

### Unit Tests
```rust
#[tokio::test]
async fn test_keepalive_heartbeat() {
    let keepalive = SseKeepAlive::new(
        Duration::from_millis(100),
        Duration::from_millis(500)
    );
    
    // Verify heartbeats are sent
    // Verify timeout detection works
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_connection_recovery() {
    // Start server
    // Connect client
    // Simulate network interruption
    // Verify automatic recovery
}
```

### Load Tests
- Simulate 100+ concurrent connections
- Random disconnections
- Verify system stability

## Toolman Integration Notes

### Required Changes
1. Toolman should handle SSE reconnection events
2. Implement exponential backoff on connection failure
3. Buffer queries during reconnection
4. Monitor heartbeat messages

### Example Toolman Configuration
```javascript
{
  "mcp_servers": {
    "doc_server": {
      "url": "http://localhost:3000",
      "transport": "sse",
      "connection": {
        "keepalive_timeout": 90,
        "reconnect_attempts": 5,
        "reconnect_delay": 1000,
        "heartbeat_interval": 30
      }
    }
  }
}
```

## Monitoring Dashboard

Key metrics to track:
- Active SSE connections
- Connection duration histogram
- Timeout events per minute
- Successful reconnections
- Failed connection attempts
- Heartbeat latency

## Backward Compatibility

All changes are additive:
- Existing clients continue to work without keep-alive
- New features are opt-in via configuration
- Protocol remains MCP-compliant
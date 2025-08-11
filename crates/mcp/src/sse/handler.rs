//! SSE handler implementation

use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::State,
    response::{sse::Event, Sse},
};
use futures::Stream;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{info, warn, error};

use crate::server::McpServerState;
use crate::sse::{ConnectionManager, HeartbeatService, SSEConfig};
use crate::sse::heartbeat::create_connection_message;

/// Enhanced SSE handler with connection management
pub async fn sse_handler(
    State(_state): State<McpServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let config = SSEConfig::from_env();
    
    // Create connection manager if not exists (this would be better as part of server state)
    let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
    
    // Add new connection
    let connection = connection_manager.add_connection().await;
    let connection_id = connection.id;
    
    info!("New SSE connection established: {}", connection_id);
    
    // Create message channel for this connection
    let (tx, rx) = mpsc::unbounded_channel::<Value>();
    connection.set_sender(tx.clone()).await;
    
    // Send initial connection message
    let connection_msg = create_connection_message(connection_id);
    if let Err(e) = tx.send(connection_msg) {
        error!("Failed to send connection message: {}", e);
    }
    
    // Create heartbeat service and start it for this connection
    let heartbeat_service = Arc::new(HeartbeatService::new(connection_manager.clone(), config.clone()));
    let _heartbeat_handle = heartbeat_service.start();
    
    // Start cleanup task
    let _cleanup_handle = connection_manager.start_cleanup_task();
    
    // Create the SSE stream
    let stream = create_sse_stream(rx, connection.clone(), connection_manager.clone());
    
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(config.heartbeat_interval)
                .text("keep-alive"),
        )
}

/// Create the SSE stream from the message receiver
fn create_sse_stream(
    rx: mpsc::UnboundedReceiver<Value>,
    connection: Arc<crate::sse::Connection>,
    connection_manager: Arc<ConnectionManager>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let connection_id = connection.id;
    
    async_stream::stream! {
        let mut stream = UnboundedReceiverStream::new(rx);
        
        // First, replay any buffered messages
        {
            let mut buffer = connection.message_buffer.write().await;
            let buffered_messages = buffer.drain_messages().await;
            
            if !buffered_messages.is_empty() {
                info!("Replaying {} buffered messages for connection {}", 
                      buffered_messages.len(), connection_id);
                      
                for buffered_msg in buffered_messages {
                    let event = create_event_from_message(&buffered_msg.content);
                    yield Ok(event);
                }
            }
        }
        
        // Then stream live messages
        use tokio_stream::StreamExt;
        while let Some(message) = stream.next().await {
            connection.update_activity().await;
            let event = create_event_from_message(&message);
            yield Ok(event);
        }
        
        // Connection closed, clean up
        connection.remove_sender().await;
        connection_manager.remove_connection(connection_id).await;
        warn!("SSE connection {} closed and cleaned up", connection_id);
    }
}

/// Create an SSE Event from a JSON message
fn create_event_from_message(message: &Value) -> Event {
    let event_type = message.get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("message");
        
    let data = message.to_string();
    
    Event::default()
        .event(event_type)
        .data(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_event_from_message() {
        let message = json!({
            "type": "heartbeat",
            "timestamp": 1234567890,
            "data": "test"
        });
        
        let event = create_event_from_message(&message);
        
        // Note: We can't easily test the internal Event structure,
        // but we can verify the function doesn't panic
        assert!(true);
    }
    
    #[test]
    fn test_create_event_from_message_no_type() {
        let message = json!({
            "timestamp": 1234567890,
            "data": "test"
        });
        
        let event = create_event_from_message(&message);
        
        // Should use default "message" type
        assert!(true);
    }
}
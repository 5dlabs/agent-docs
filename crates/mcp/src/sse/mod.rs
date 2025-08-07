//! SSE (Server-Sent Events) module for keep-alive connections
//!
//! This module provides comprehensive SSE functionality including:
//! - Connection management and tracking
//! - Heartbeat mechanism with configurable intervals
//! - Message buffering during disconnections
//! - Connection state monitoring and metrics

pub mod connection;
pub mod handler;
pub mod heartbeat;
pub mod buffer;
pub mod config;

pub use connection::{Connection, ConnectionManager};
pub use handler::sse_handler;
pub use heartbeat::HeartbeatService;
pub use buffer::MessageBuffer;
pub use config::SSEConfig;

#[cfg(test)]
mod tests;
//! MCP (Model Context Protocol) server implementation
//!
//! This crate provides the MCP server functionality including tool definitions,
//! HTTP/SSE transport, and integration with the database and other services.

pub mod handlers;
pub mod headers;
pub mod server;
pub mod tools;
pub mod transport;
// pub mod sse;  // TODO: implement SSE module

pub use server::McpServer;

/// Re-export commonly used types
pub use rmcp::*;

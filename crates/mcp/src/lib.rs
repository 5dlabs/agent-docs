//! MCP (Model Context Protocol) server implementation
//!
//! This crate provides the MCP server functionality including tool definitions,
//! HTTP transport, and integration with the database and other services.

pub mod handlers;
pub mod headers;
pub mod health;
pub mod security;
pub mod server;
pub mod session;
pub mod tools;
pub mod transport;

pub use server::McpServer;

// Re-export commonly used types
// pub use brk_rmcp as rmcp;  // Temporarily disabled due to edition2024 requirement
// pub use rmcp::*;

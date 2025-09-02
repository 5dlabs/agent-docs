//! MCP (Model Context Protocol) server implementation
//!
//! This crate provides the MCP server functionality including tool definitions,
//! HTTP transport, and integration with the database and other services.
//!
//! Test deployment with namespace fix applied.

pub mod config;
pub mod crate_tools;
pub mod handlers;
pub mod headers;
pub mod health;
pub mod job_queue;
pub mod metrics;
pub mod protocol_version;
pub mod security;
pub mod server;
pub mod session;
pub mod tools;
pub mod transport;

pub use server::McpServer;

// Re-export commonly used types
// pub use brk_rmcp as rmcp;  // Temporarily disabled due to edition2024 requirement
// pub use rmcp::*;
// Force rebuild Mon Sep  1 18:42:21 PDT 2025
// Force rebuild 1756778257

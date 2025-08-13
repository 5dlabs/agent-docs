//! MCP transport layer (MVP scaffold)

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    response::Response,
};
use thiserror::Error;

use crate::server::McpServerState;

#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub protocol_version: String,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("method not allowed")]
    MethodNotAllowed,
}

pub async fn unified_mcp_handler(
    State(_state): State<McpServerState>,
    _headers: HeaderMap,
    _request: Request,
) -> Result<Response, TransportError> {
    Err(TransportError::MethodNotAllowed)
}

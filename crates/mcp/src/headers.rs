//! MCP header constants and helpers (MVP)

use axum::http::{HeaderMap, HeaderValue, StatusCode};
use uuid::Uuid;

pub const MCP_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";
pub const MCP_SESSION_ID: &str = "Mcp-Session-Id";
pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";

/// Validate that incoming request headers include the supported MCP protocol version.
/// Returns 400 Bad Request if the header is missing or has an unsupported value.
pub fn validate_protocol_version(headers: &HeaderMap) -> Result<(), StatusCode> {
    match headers.get(MCP_PROTOCOL_VERSION) {
        Some(value) => {
            if let Ok(v) = value.to_str() {
                if v == SUPPORTED_PROTOCOL_VERSION {
                    return Ok(());
                }
            }
            Err(StatusCode::BAD_REQUEST)
        }
        None => Err(StatusCode::BAD_REQUEST),
    }
}

/// Set standard MCP headers on the provided response headers.
pub fn set_standard_headers(headers: &mut HeaderMap, session_id: Option<Uuid>) {
    headers.insert(
        MCP_PROTOCOL_VERSION,
        HeaderValue::from_static(SUPPORTED_PROTOCOL_VERSION),
    );
    if let Some(id) = session_id {
        if let Ok(v) = HeaderValue::from_str(&id.to_string()) {
            headers.insert(MCP_SESSION_ID, v);
        }
    }
}



//! MCP header constants and validators

use axum::http::{HeaderMap, StatusCode};
use uuid::Uuid;

/// MCP Protocol Version header name
pub const MCP_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";

/// MCP Session ID header name  
pub const MCP_SESSION_ID: &str = "Mcp-Session-Id";

/// The supported protocol version
pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";

/// Validates the MCP protocol version header
/// Returns Ok(()) if the version is supported, Err(StatusCode) otherwise
pub fn validate_protocol_version(headers: &HeaderMap) -> Result<(), StatusCode> {
    match headers.get(MCP_PROTOCOL_VERSION) {
        Some(version_header) => {
            match version_header.to_str() {
                Ok(version) if version == SUPPORTED_PROTOCOL_VERSION => Ok(()),
                Ok(version) => {
                    tracing::warn!("Unsupported protocol version: {}", version);
                    Err(StatusCode::BAD_REQUEST)
                }
                Err(_) => {
                    tracing::warn!("Invalid protocol version header format");
                    Err(StatusCode::BAD_REQUEST)
                }
            }
        }
        None => {
            tracing::warn!("Missing MCP protocol version header");
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Sets standard MCP headers on a response
/// Adds the protocol version and optionally a session ID if provided
pub fn set_standard_headers(headers: &mut HeaderMap, session_id: Option<Uuid>) {
    headers.insert(
        MCP_PROTOCOL_VERSION,
        SUPPORTED_PROTOCOL_VERSION.parse().expect("Valid protocol version"),
    );
    
    if let Some(session_id) = session_id {
        headers.insert(
            MCP_SESSION_ID,
            session_id.to_string().parse().expect("Valid UUID string"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_validate_protocol_version_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            MCP_PROTOCOL_VERSION,
            HeaderValue::from_static(SUPPORTED_PROTOCOL_VERSION),
        );

        assert!(validate_protocol_version(&headers).is_ok());
    }

    #[test]
    fn test_validate_protocol_version_unsupported() {
        let mut headers = HeaderMap::new();
        headers.insert(
            MCP_PROTOCOL_VERSION,
            HeaderValue::from_static("2024-11-05"),
        );

        assert_eq!(
            validate_protocol_version(&headers),
            Err(StatusCode::BAD_REQUEST)
        );
    }

    #[test]
    fn test_validate_protocol_version_missing() {
        let headers = HeaderMap::new();
        assert_eq!(
            validate_protocol_version(&headers),
            Err(StatusCode::BAD_REQUEST)
        );
    }

    #[test]
    fn test_set_standard_headers() {
        let mut headers = HeaderMap::new();
        let session_id = Uuid::new_v4();

        set_standard_headers(&mut headers, Some(session_id));

        assert_eq!(
            headers.get(MCP_PROTOCOL_VERSION).unwrap(),
            SUPPORTED_PROTOCOL_VERSION
        );
        assert_eq!(
            headers.get(MCP_SESSION_ID).unwrap().to_str().unwrap(),
            session_id.to_string()
        );
    }

    #[test]
    fn test_set_standard_headers_no_session() {
        let mut headers = HeaderMap::new();

        set_standard_headers(&mut headers, None);

        assert_eq!(
            headers.get(MCP_PROTOCOL_VERSION).unwrap(),
            SUPPORTED_PROTOCOL_VERSION
        );
        assert!(headers.get(MCP_SESSION_ID).is_none());
    }
}
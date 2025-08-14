//! MCP header constants, extractors, and helpers
//!
//! This module provides comprehensive header handling for the MCP protocol,
//! including Axum extractors for validation and standardized response header management.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::str::FromStr;
use thiserror::Error;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::protocol_version::ProtocolRegistry;

/// MCP protocol version header name
pub const MCP_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";
/// MCP session ID header name  
pub const MCP_SESSION_ID: &str = "Mcp-Session-Id";
/// The only supported protocol version (fixed for MVP)
pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";
/// Content-Type for JSON responses
pub const CONTENT_TYPE_JSON: &str = "application/json";
/// Content-Type for Server-Sent Events (future use)
pub const CONTENT_TYPE_SSE: &str = "text/event-stream";

/// Protocol version validation errors
#[derive(Debug, Error)]
pub enum ProtocolVersionError {
    #[error("Missing MCP-Protocol-Version header")]
    MissingHeader,
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(String),
    #[error("Unsupported protocol version: {0} (only {1} supported)")]
    UnsupportedVersion(String, String),
}

impl IntoResponse for ProtocolVersionError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProtocolVersionError::MissingHeader => (
                StatusCode::BAD_REQUEST,
                "Missing MCP-Protocol-Version header",
            ),
            ProtocolVersionError::InvalidHeaderValue(_) => (
                StatusCode::BAD_REQUEST,
                "Invalid MCP-Protocol-Version header value",
            ),
            ProtocolVersionError::UnsupportedVersion(_, _) => {
                (StatusCode::BAD_REQUEST, "Unsupported protocol version")
            }
        };

        let error_response = json!({
            "error": {
                "code": -32600,
                "message": message,
                "data": self.to_string()
            }
        });

        let mut headers = HeaderMap::new();
        set_standard_headers(&mut headers, None);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_JSON));

        (status, headers, axum::Json(error_response)).into_response()
    }
}

/// Axum extractor for MCP Protocol Version header validation
///
/// This extractor validates that the incoming request has the correct MCP-Protocol-Version
/// header with the supported version (2025-06-18 only).
#[derive(Debug, Clone)]
pub struct McpProtocolVersionHeader {
    /// The validated protocol version (always "2025-06-18" if extraction succeeds)
    pub version: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for McpProtocolVersionHeader
where
    S: Send + Sync,
{
    type Rejection = ProtocolVersionError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;
        let registry = ProtocolRegistry::new();

        debug!("Validating MCP protocol version header");

        if let Some(value) = headers.get(MCP_PROTOCOL_VERSION) {
            let version_str = value.to_str().map_err(|_| {
                ProtocolVersionError::InvalidHeaderValue("non-UTF8 value".to_string())
            })?;

            debug!("Found protocol version header: {version_str}");

            // Use protocol registry for validation
            if registry.validate_version_string(version_str).is_ok() {
                Ok(McpProtocolVersionHeader {
                    version: version_str.to_string(),
                })
            } else {
                warn!("Unsupported protocol version requested: {version_str}");
                Err(ProtocolVersionError::UnsupportedVersion(
                    version_str.to_string(),
                    registry.current_version_string().to_string(),
                ))
            }
        } else {
            warn!("Missing MCP-Protocol-Version header");
            Err(ProtocolVersionError::MissingHeader)
        }
    }
}

/// Content-Type validation errors
#[derive(Debug, Error)]
pub enum ContentTypeError {
    #[error("Missing Content-Type header")]
    MissingHeader,
    #[error("Invalid Content-Type header value")]
    InvalidHeaderValue,
    #[error("Unsupported Content-Type: {0}")]
    UnsupportedContentType(String),
}

/// Accept header validation errors
#[derive(Debug, Error)]
pub enum AcceptHeaderError {
    #[error("Invalid Accept header value")]
    InvalidHeaderValue,
    #[error("Unacceptable media type: {0}")]
    UnacceptableMediaType(String),
}

impl IntoResponse for ContentTypeError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ContentTypeError::MissingHeader => {
                (StatusCode::BAD_REQUEST, "Missing Content-Type header")
            }
            ContentTypeError::InvalidHeaderValue => {
                (StatusCode::BAD_REQUEST, "Invalid Content-Type header value")
            }
            ContentTypeError::UnsupportedContentType(_) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Unsupported Content-Type",
            ),
        };

        let error_response = json!({
            "error": {
                "code": -32600,
                "message": message,
                "data": self.to_string()
            }
        });

        let mut headers = HeaderMap::new();
        set_standard_headers(&mut headers, None);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_JSON));

        (status, headers, axum::Json(error_response)).into_response()
    }
}

impl IntoResponse for AcceptHeaderError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AcceptHeaderError::InvalidHeaderValue => {
                (StatusCode::BAD_REQUEST, "Invalid Accept header value")
            }
            AcceptHeaderError::UnacceptableMediaType(_) => {
                (StatusCode::NOT_ACCEPTABLE, "Not Acceptable")
            }
        };

        let error_response = json!({
            "error": {
                "code": -32600,
                "message": message,
                "data": self.to_string()
            }
        });

        let mut headers = HeaderMap::new();
        set_standard_headers(&mut headers, None);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_JSON));

        (status, headers, axum::Json(error_response)).into_response()
    }
}

/// Axum extractor for Content-Type header validation
///
/// Validates that the request has an appropriate Content-Type header for MCP operations.
#[derive(Debug, Clone)]
pub struct ContentTypeValidator {
    /// The validated content type
    pub content_type: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for ContentTypeValidator
where
    S: Send + Sync,
{
    type Rejection = ContentTypeError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        debug!("Validating Content-Type header");

        if let Some(value) = headers.get(CONTENT_TYPE) {
            let content_type = value
                .to_str()
                .map_err(|_| ContentTypeError::InvalidHeaderValue)?;

            debug!("Found Content-Type header: {content_type}");

            // Accept application/json and text/event-stream
            if content_type.starts_with("application/json")
                || content_type.starts_with("text/event-stream")
            {
                Ok(ContentTypeValidator {
                    content_type: content_type.to_string(),
                })
            } else {
                warn!("Unsupported Content-Type: {content_type}");
                Err(ContentTypeError::UnsupportedContentType(
                    content_type.to_string(),
                ))
            }
        } else {
            warn!("Missing Content-Type header");
            Err(ContentTypeError::MissingHeader)
        }
    }
}

/// Axum extractor for Accept header validation
///
/// Validates that the request accepts compatible content types for MCP responses.
#[derive(Debug, Clone)]
pub struct AcceptHeaderValidator {
    /// The acceptable content types
    pub accept_types: Vec<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for AcceptHeaderValidator
where
    S: Send + Sync,
{
    type Rejection = AcceptHeaderError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        debug!("Validating Accept header");

        if let Some(value) = headers.get("accept") {
            let accept_header = value
                .to_str()
                .map_err(|_| AcceptHeaderError::InvalidHeaderValue)?;

            debug!("Found Accept header: {accept_header}");

            // Parse Accept header to check for compatible media types
            // Accept application/json, application/*, or */*
            let acceptable_types = vec![
                "application/json".to_string(),
                "application/*".to_string(),
                "*/*".to_string(),
                "text/event-stream".to_string(), // For future SSE support
            ];

            // Check if any of our supported types match the Accept header
            for acceptable_type in &acceptable_types {
                if accept_header.contains(acceptable_type)
                    || accept_header.contains("application/*")
                    || accept_header.contains("*/*")
                {
                    return Ok(AcceptHeaderValidator {
                        accept_types: vec![accept_header.to_string()],
                    });
                }
            }

            warn!("Unacceptable Accept header: {accept_header}");
            Err(AcceptHeaderError::UnacceptableMediaType(
                accept_header.to_string(),
            ))
        } else {
            // Missing Accept header is acceptable (defaults to accepting anything)
            debug!("No Accept header provided - defaulting to JSON");
            Ok(AcceptHeaderValidator {
                accept_types: vec!["application/json".to_string()],
            })
        }
    }
}

/// Validate that incoming request headers include the supported MCP protocol version.
///
/// This function validates the MCP-Protocol-Version header using the protocol registry
/// to ensure only the supported version (2025-06-18) is accepted.
///
/// # Errors
///
/// Returns `Err(StatusCode::BAD_REQUEST)` if the header is missing or has an
/// unsupported value.
pub fn validate_protocol_version(headers: &HeaderMap) -> Result<(), StatusCode> {
    let registry = ProtocolRegistry::new();

    if let Some(value) = headers.get(MCP_PROTOCOL_VERSION) {
        if let Ok(version_str) = value.to_str() {
            if registry.is_version_string_supported(version_str) {
                Ok(())
            } else {
                debug!("Unsupported protocol version: {}", version_str);
                Err(StatusCode::BAD_REQUEST)
            }
        } else {
            warn!("Invalid protocol version header value");
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        warn!("Missing MCP-Protocol-Version header");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Set standard MCP headers on the provided response headers.
///
/// This function adds the MCP-Protocol-Version header (fixed to supported version)
/// and optionally the Mcp-Session-Id header if a session ID is provided.
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

/// Set response headers for JSON responses
///
/// This is a convenience function that sets both standard MCP headers
/// and the appropriate Content-Type for JSON responses.
pub fn set_json_response_headers(headers: &mut HeaderMap, session_id: Option<Uuid>) {
    set_standard_headers(headers, session_id);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_JSON));
}

/// Set response headers for Server-Sent Events responses (future use)
///
/// This function sets headers appropriate for SSE responses including
/// the standard MCP headers and SSE-specific headers.
#[allow(dead_code)]
pub fn set_sse_response_headers(headers: &mut HeaderMap, session_id: Option<Uuid>) {
    set_standard_headers(headers, session_id);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_SSE));
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers.insert("Connection", HeaderValue::from_static("keep-alive"));
}

/// Extract session ID from request headers
///
/// # Errors
///
/// Returns an error if the session ID header is present but malformed.
pub fn extract_session_id(headers: &HeaderMap) -> Result<Option<Uuid>, String> {
    match headers.get(MCP_SESSION_ID) {
        Some(value) => {
            let session_str = value
                .to_str()
                .map_err(|_| "Invalid session ID header value".to_string())?;
            let session_id = Uuid::from_str(session_str)
                .map_err(|_| format!("Invalid session ID format: {session_str}"))?;
            Ok(Some(session_id))
        }
        None => Ok(None),
    }
}

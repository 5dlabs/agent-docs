//! MCP transport layer - Streamable HTTP transport implementation
//!
//! This module implements the MCP 2025-06-18 Streamable HTTP transport protocol.
//! It provides session management, protocol version validation, and unified endpoint handling.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::headers::{
    set_json_response_headers, set_standard_headers, validate_protocol_version, MCP_SESSION_ID,
    SUPPORTED_PROTOCOL_VERSION,
};
use crate::security::{add_security_headers, validate_dns_rebinding, validate_origin};
use crate::server::McpServerState;
use crate::session::ClientInfo;

/// Transport configuration
#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub protocol_version: String,
    pub session_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub max_json_body_bytes: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            protocol_version: "2025-06-18".to_string(),
            session_timeout: Duration::from_secs(300), // 5 minutes
            heartbeat_interval: Duration::from_secs(30), // 30 seconds
            max_json_body_bytes: 2 * 1024 * 1024, // 2 MiB default, matching Axum's default body limit
        }
    }
}

/// Session identifier type
pub type SessionId = Uuid;

/// SSE message structure for future streaming support
#[derive(Debug, Clone)]
pub struct SseMessage {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
}

/// MCP session state
#[derive(Debug, Clone)]
pub struct McpSession {
    pub id: SessionId,
    pub created_at: Instant,
    pub last_activity: Arc<RwLock<Instant>>,
    pub message_sender: broadcast::Sender<SseMessage>,
}

impl McpSession {
    /// Create a new session
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        let now = Instant::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
            message_sender: sender,
        }
    }

    /// Update session activity timestamp
    pub fn update_activity(&self) {
        if let Ok(mut last_activity) = self.last_activity.write() {
            *last_activity = Instant::now();
        }
    }

    /// Check if session has expired
    #[must_use]
    pub fn is_expired(&self, timeout: Duration) -> bool {
        if let Ok(last_activity) = self.last_activity.read() {
            last_activity.elapsed() > timeout
        } else {
            false
        }
    }
}

impl Default for McpSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Session manager for handling MCP sessions
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, McpSession>>>,
    config: TransportConfig,
}

impl SessionManager {
    /// Create a new session manager
    #[must_use]
    pub fn new(config: TransportConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new session
    ///
    /// # Errors
    ///
    /// Returns an error if the internal session map cannot be locked for writing.
    pub fn create_session(&self) -> Result<SessionId, TransportError> {
        let session = McpSession::new();
        let session_id = session.id;

        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| TransportError::SessionLockError)?;
        sessions.insert(session_id, session);

        debug!("Created new session: {}", session_id);
        Ok(session_id)
    }

    /// Get or create session from headers
    /// Get an existing session from headers or create a new one.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal session map cannot be accessed.
    pub fn get_or_create_session(&self, headers: &HeaderMap) -> Result<SessionId, TransportError> {
        // Try to extract session ID from headers
        if let Some(session_header) = headers.get(MCP_SESSION_ID) {
            if let Ok(session_str) = session_header.to_str() {
                if let Ok(session_id) = Uuid::parse_str(session_str) {
                    // Check if session exists and is valid
                    if let Ok(sessions) = self.sessions.read() {
                        if let Some(session) = sessions.get(&session_id) {
                            if session.is_expired(self.config.session_timeout) {
                                debug!("Session expired: {}", session_id);
                            } else {
                                session.update_activity();
                                debug!("Using existing session: {}", session_id);
                                return Ok(session_id);
                            }
                        }
                    }
                }
            }
        }

        // Create new session if none found or existing is invalid
        self.create_session()
    }

    /// Update session activity
    /// Update session activity timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the session does not exist or the map cannot be read.
    pub fn update_session_activity(&self, session_id: SessionId) -> Result<(), TransportError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| TransportError::SessionLockError)?;
        if let Some(session) = sessions.get(&session_id) {
            session.update_activity();
            Ok(())
        } else {
            Err(TransportError::SessionNotFound(session_id))
        }
    }

    /// Clean up expired sessions
    /// Cleanup expired sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if the session map cannot be locked for writing.
    pub fn cleanup_expired_sessions(&self) -> Result<usize, TransportError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| TransportError::SessionLockError)?;
        let initial_count = sessions.len();

        sessions.retain(|_id, session| !session.is_expired(self.config.session_timeout));

        let cleaned_count = initial_count - sessions.len();
        if cleaned_count > 0 {
            debug!("Cleaned up {} expired sessions", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get session count for monitoring
    /// Get current number of sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if the session map cannot be accessed.
    pub fn session_count(&self) -> Result<usize, TransportError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| TransportError::SessionLockError)?;
        Ok(sessions.len())
    }
}

/// Transport-specific error types
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Method not allowed")]
    MethodNotAllowed,

    #[error("Unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(String),

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Session lock error")]
    SessionLockError,

    #[error("Missing content type")]
    MissingContentType,

    #[error("Invalid content type: {0}")]
    InvalidContentType(String),

    #[error("JSON parsing error: {0}")]
    JsonParseError(String),

    #[error("Payload too large")]
    PayloadTooLarge,

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Security validation failed: {0}")]
    SecurityValidationFailed(String),

    #[error("Invalid Accept header: {0}")]
    InvalidAcceptHeader(String),

    #[error("Unacceptable Accept header: {0}")]
    UnacceptableAcceptHeader(String),
}

impl IntoResponse for TransportError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            TransportError::MethodNotAllowed => {
                (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
            }
            TransportError::UnsupportedProtocolVersion(_) => {
                (StatusCode::BAD_REQUEST, "Unsupported Protocol Version")
            }
            TransportError::SessionNotFound(_) => (StatusCode::BAD_REQUEST, "Session Not Found"),
            TransportError::InvalidSessionId(_) => (StatusCode::BAD_REQUEST, "Invalid Session ID"),
            TransportError::SessionLockError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Session Lock Error")
            }
            TransportError::MissingContentType => (StatusCode::BAD_REQUEST, "Missing Content-Type"),
            TransportError::InvalidContentType(_) => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type")
            }
            TransportError::JsonParseError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON"),
            TransportError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large"),
            TransportError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            TransportError::SecurityValidationFailed(_) => {
                (StatusCode::FORBIDDEN, "Security Validation Failed")
            }
            TransportError::InvalidAcceptHeader(_) => {
                (StatusCode::BAD_REQUEST, "Invalid Accept Header")
            }
            TransportError::UnacceptableAcceptHeader(_) => {
                (StatusCode::NOT_ACCEPTABLE, "Not Acceptable")
            }
        };

        error!("Transport error: {}", self);

        let error_response = json!({
            "error": {
                "code": -32600,
                "message": error_message,
                "data": self.to_string()
            }
        });

        let mut headers = HeaderMap::new();
        set_json_response_headers(&mut headers, None);

        (status, headers, Json(error_response)).into_response()
    }
}

/// Unified MCP endpoint handler supporting both POST (JSON) and GET (SSE) - MVP: POST only
///
/// This handler processes all MCP requests according to the 2025-06-18 specification:
/// - POST requests with application/json -> JSON-RPC processing
/// - GET requests -> 405 Method Not Allowed (MVP does not support SSE)
///   Unified MCP endpoint handler.
///
/// # Errors
///
/// Returns a `TransportError` when protocol validation fails, when the request
/// uses an unsupported method, or when JSON parsing/processing fails.
pub async fn unified_mcp_handler(
    State(state): State<McpServerState>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response, TransportError> {
    debug!(
        "Received MCP request: {:?} {}",
        request.method(),
        request.uri()
    );

    // Validate protocol version first
    if let Err(status) = validate_protocol_version(&headers) {
        return match status {
            StatusCode::BAD_REQUEST => Err(TransportError::UnsupportedProtocolVersion(
                headers
                    .get("MCP-Protocol-Version")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("missing")
                    .to_string(),
            )),
            _ => Err(TransportError::InternalError(
                "Protocol validation failed".to_string(),
            )),
        };
    }

    // Validate Accept header for method compatibility (JSON-only policy)
    // Note: For GET, SSE is disabled, so we skip Accept validation and return 405 below.
    validate_accept_header(&headers, request.method())?;

    match *request.method() {
        Method::POST => handle_json_rpc_request(state, headers, request).await,
        Method::DELETE => handle_delete_session_request(&state, &headers),
        Method::GET => {
            // JSON-only policy: SSE disabled. Always return 405 regardless of Accept header.
            warn!("GET request to /mcp endpoint - returning 405 Method Not Allowed");
            Err(TransportError::MethodNotAllowed)
        }
        _ => {
            warn!("Unsupported HTTP method: {}", request.method());
            Err(TransportError::MethodNotAllowed)
        }
    }
}

/// Validate Accept header for the given request method and headers
///
/// # Errors
///
/// Returns `TransportError` if the Accept header is unacceptable for the request method.
fn validate_accept_header(headers: &HeaderMap, method: &Method) -> Result<(), TransportError> {
    if let Some(value) = headers.get("accept") {
        if let Ok(accept_header) = value.to_str() {
            debug!("Validating Accept header: {accept_header}");

            // For POST (JSON-RPC) requests, Accept should be compatible with application/json
            match *method {
                Method::POST => {
                    if accept_header.contains("application/json")
                        || accept_header.contains("application/*")
                        || accept_header.contains("*/*")
                    {
                        Ok(())
                    } else {
                        warn!("Unacceptable Accept header for POST: {accept_header}");
                        Err(TransportError::UnacceptableAcceptHeader(
                            accept_header.to_string(),
                        ))
                    }
                }
                Method::GET => {
                    // SSE disabled: skip Accept validation for GET. Handler returns 405.
                    Ok(())
                }
                _ => Ok(()), // Other methods don't have specific Accept requirements
            }
        } else {
            warn!("Invalid Accept header value");
            Err(TransportError::InvalidAcceptHeader(
                "invalid header value".to_string(),
            ))
        }
    } else {
        // Missing Accept header is acceptable (defaults based on method)
        debug!("No Accept header provided - defaulting based on method");
        Ok(())
    }
}

/// Extract client information from request headers
fn extract_client_info(headers: &HeaderMap) -> ClientInfo {
    ClientInfo {
        user_agent: headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
        origin: headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
        ip_address: None, // IP address would come from connection info if needed
    }
}

/// Get or create session using the comprehensive session manager
///
/// # Errors
///
/// Returns `TransportError` if session operations fail.
fn get_or_create_comprehensive_session(
    state: &McpServerState,
    headers: &HeaderMap,
    client_info: Option<ClientInfo>,
) -> Result<Uuid, TransportError> {
    // Try to extract session ID from headers
    if let Some(session_header) = headers.get(MCP_SESSION_ID) {
        if let Ok(session_str) = session_header.to_str() {
            if let Ok(session_id) = Uuid::parse_str(session_str) {
                // Check if session exists in comprehensive session manager
                if let Ok(session) = state.comprehensive_session_manager.get_session(session_id) {
                    if session.is_expired() {
                        debug!("Comprehensive session expired: {}", session_id);
                    } else if let Err(e) =
                        session.validate_protocol_version(SUPPORTED_PROTOCOL_VERSION)
                    {
                        warn!("Session protocol version mismatch: {}", e);
                        debug!(
                            "Invalidating session with wrong protocol version: {}",
                            session_id
                        );
                    } else {
                        // Update session activity
                        let _ = state
                            .comprehensive_session_manager
                            .update_last_accessed(session_id);
                        debug!("Using existing comprehensive session: {}", session_id);
                        return Ok(session_id);
                    }
                }
            }
        }
    }

    // Create new session if none found or existing is invalid
    let session_id = state
        .comprehensive_session_manager
        .create_session(client_info)
        .map_err(|e| TransportError::InternalError(format!("Session creation failed: {e}")))?;

    debug!("Created new comprehensive session: {}", session_id);
    Ok(session_id)
}

/// Handle DELETE requests for explicit session termination
fn handle_delete_session_request(
    state: &McpServerState,
    headers: &HeaderMap,
) -> Result<Response, TransportError> {
    debug!("Processing DELETE session request");

    // Security validation first
    if let Err(e) = validate_origin(headers, &state.security_config) {
        error!("Origin validation failed for DELETE: {}", e);
        return Err(TransportError::SecurityValidationFailed(e.to_string()));
    }

    if let Err(e) = validate_dns_rebinding(headers, &state.security_config) {
        error!("DNS rebinding validation failed for DELETE: {}", e);
        return Err(TransportError::SecurityValidationFailed(e.to_string()));
    }

    // Extract session ID from headers
    if let Some(session_header) = headers.get(MCP_SESSION_ID) {
        if let Ok(session_str) = session_header.to_str() {
            if let Ok(session_id) = Uuid::parse_str(session_str) {
                // Attempt to delete the session
                if state
                    .comprehensive_session_manager
                    .delete_session(session_id)
                    .is_ok()
                {
                    debug!("Successfully deleted session: {}", session_id);

                    // Create response with proper headers
                    let mut response_headers = HeaderMap::new();
                    set_standard_headers(&mut response_headers, Some(session_id));
                    add_security_headers(&mut response_headers);

                    Ok((StatusCode::NO_CONTENT, response_headers, "").into_response())
                } else {
                    debug!("Session not found for deletion: {}", session_id);

                    // Return 404 for non-existent sessions
                    let mut response_headers = HeaderMap::new();
                    set_standard_headers(&mut response_headers, None);
                    add_security_headers(&mut response_headers);

                    Ok((StatusCode::NOT_FOUND, response_headers, "").into_response())
                }
            } else {
                warn!(
                    "Invalid session ID format in DELETE request: {}",
                    session_str
                );
                Err(TransportError::InvalidSessionId(session_str.to_string()))
            }
        } else {
            warn!("Invalid session header value in DELETE request");
            Err(TransportError::InvalidSessionId(
                "invalid header value".to_string(),
            ))
        }
    } else {
        warn!("Missing session ID in DELETE request");
        Err(TransportError::InvalidSessionId(
            "missing session header".to_string(),
        ))
    }
}

/// Handle JSON-RPC requests over HTTP POST
async fn handle_json_rpc_request(
    state: McpServerState,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response, TransportError> {
    debug!("Processing JSON-RPC request");

    // Security validation first
    if let Err(e) = validate_origin(&headers, &state.security_config) {
        error!("Origin validation failed: {}", e);
        return Err(TransportError::SecurityValidationFailed(e.to_string()));
    }

    if let Err(e) = validate_dns_rebinding(&headers, &state.security_config) {
        error!("DNS rebinding validation failed: {}", e);
        return Err(TransportError::SecurityValidationFailed(e.to_string()));
    }

    // Validate Content-Type
    let content_type = headers
        .get("content-type")
        .ok_or(TransportError::MissingContentType)?
        .to_str()
        .map_err(|_| TransportError::InvalidContentType("invalid header value".to_string()))?;

    if !content_type.starts_with("application/json") {
        return Err(TransportError::InvalidContentType(content_type.to_string()));
    }

    // Extract client information for session management
    let client_info = extract_client_info(&headers);

    // Get or create session using the comprehensive session manager
    let session_id = get_or_create_comprehensive_session(&state, &headers, Some(client_info))?;

    // Enforce a maximum body size similar to Axum's Json extractor default
    let max_body_bytes = state.transport_config.max_json_body_bytes;

    // If Content-Length is present and exceeds the limit, reject early
    if let Some(len_header) = headers.get("content-length") {
        if let Ok(len_str) = len_header.to_str() {
            if let Ok(len) = len_str.parse::<u64>() {
                if len > max_body_bytes as u64 {
                    return Err(TransportError::PayloadTooLarge);
                }
            }
        }
    }

    // Extract request body with an explicit limit
    let body_bytes = axum::body::to_bytes(request.into_body(), max_body_bytes)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            let lower = msg.to_ascii_lowercase();
            if lower.contains("length") && lower.contains("limit") || lower.contains("too large") {
                TransportError::PayloadTooLarge
            } else {
                TransportError::InternalError(format!("Failed to read body: {msg}"))
            }
        })?;

    // Parse JSON-RPC request
    let json_request: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| TransportError::JsonParseError(e.to_string()))?;

    debug!("Parsed JSON-RPC request: {}", json_request);

    // Process through existing MCP handler
    match state.handler.handle_request(json_request).await {
        Ok(response) => {
            debug!("MCP handler response: {}", response);

            // Update session activity using comprehensive session manager
            let _ = state
                .comprehensive_session_manager
                .update_last_accessed(session_id);

            // Create response with proper headers
            let mut response_headers = HeaderMap::new();
            set_json_response_headers(&mut response_headers, Some(session_id));
            add_security_headers(&mut response_headers);

            Ok((StatusCode::OK, response_headers, Json(response)).into_response())
        }
        Err(e) => {
            error!("MCP handler failed: {}", e);
            Err(TransportError::InternalError(format!("Handler error: {e}")))
        }
    }
}

/// Initialize transport with session cleanup task
///
/// This function starts a background task that periodically cleans up expired sessions.
/// It should be called during server startup.
pub async fn initialize_transport(session_manager: SessionManager) {
    let cleanup_interval = Duration::from_secs(60); // Cleanup every minute
    let manager = session_manager.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(cleanup_interval);

        loop {
            interval.tick().await;

            match manager.cleanup_expired_sessions() {
                Ok(cleaned) => {
                    if cleaned > 0 {
                        debug!("Session cleanup: removed {} expired sessions", cleaned);
                    }
                }
                Err(e) => {
                    error!("Session cleanup failed: {}", e);
                }
            }
        }
    });

    debug!("Transport initialized with session cleanup task");
}

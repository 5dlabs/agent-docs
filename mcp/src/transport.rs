//! MCP transport layer - Streamable HTTP transport implementation
//!
//! This module implements the MCP 2025-06-18 Streamable HTTP transport protocol.
//! It provides session management, protocol version validation, and unified endpoint handling.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn, Instrument};
use uuid::Uuid;

// Sensitive header names (lowercase) to redact in detailed logs
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-api-key-id",
];

use crate::headers::{
    set_json_response_headers, set_standard_headers, validate_protocol_version, MCP_SESSION_ID,
    SUPPORTED_PROTOCOL_VERSION,
};
use crate::metrics::metrics;
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
            session_timeout: Duration::from_secs(1800), // 30 minutes for SSE connections
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

/// In-memory SSE hub for per-session message broadcasting with replay buffer.
#[derive(Debug)]
struct SessionStream {
    sender: broadcast::Sender<SseMessage>,
    buffer: VecDeque<(u64, SseMessage)>,
    next_id: u64,
}

impl SessionStream {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            sender,
            buffer: VecDeque::with_capacity(256),
            next_id: 1,
        }
    }
}

#[derive(Debug, Default)]
struct SseHub {
    sessions: RwLock<HashMap<Uuid, Arc<RwLock<SessionStream>>>>,
}

impl SseHub {
    const MAX_BUFFER: usize = 256;

    fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    fn get_or_create(&self, session_id: Uuid) -> Arc<RwLock<SessionStream>> {
        if let Ok(map) = self.sessions.read() {
            if let Some(s) = map.get(&session_id) {
                return Arc::clone(s);
            }
        }
        let new_stream = Arc::new(RwLock::new(SessionStream::new()));
        if let Ok(mut map) = self.sessions.write() {
            let entry = map.entry(session_id).or_insert_with(|| Arc::clone(&new_stream));
            Arc::clone(entry)
        } else {
            new_stream
        }
    }

    fn subscribe(&self, session_id: Uuid) -> broadcast::Receiver<SseMessage> {
        let s = self.get_or_create(session_id);
        s.read()
            .map(|ss| ss.sender.subscribe())
            .unwrap_or_else(|_| {
                let (tx, _rx) = broadcast::channel(1);
                tx.subscribe()
            })
    }

    fn snapshot_from(&self, session_id: Uuid, last_id: Option<u64>) -> Vec<(u64, SseMessage)> {
        let s = self.get_or_create(session_id);
        let Ok(ss) = s.read() else { return vec![] };
        let start_after = last_id.unwrap_or(0);
        ss.buffer
            .iter()
            .filter(|(id, _)| *id > start_after)
            .cloned()
            .collect()
    }

    fn publish(&self, session_id: Uuid, mut msg: SseMessage) -> u64 {
        let s = self.get_or_create(session_id);
        let mut id_assigned = 0;
        if let Ok(mut ss) = s.write() {
            let id = ss.next_id;
            ss.next_id = ss.next_id.saturating_add(1);
            msg.id = Some(id.to_string());
            // buffer and trim
            ss.buffer.push_back((id, msg.clone()));
            while ss.buffer.len() > Self::MAX_BUFFER {
                ss.buffer.pop_front();
            }
            // best-effort broadcast
            let _ = ss.sender.send(msg);
            id_assigned = id;
        }
        id_assigned
    }
}

static SSE_HUB: std::sync::LazyLock<SseHub> = std::sync::LazyLock::new(SseHub::new);

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
        self.last_activity
            .read()
            .is_ok_and(|last_activity| last_activity.elapsed() > timeout)
    }
}

impl Default for McpSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Redact and format headers for logging
fn log_request_headers(request_id: Uuid, headers: &HeaderMap) {
    // Helper to fetch header values as UTF-8 strings
    fn header_str(headers: &HeaderMap, name: &str) -> String {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map_or_else(|| "<missing>".to_string(), ToString::to_string)
    }

    // Short summary at info level for quick visibility
    let proto = header_str(headers, "MCP-Protocol-Version");
    let accept = header_str(headers, "accept");
    let content_type = header_str(headers, "content-type");
    let session_id = header_str(headers, MCP_SESSION_ID);
    let user_agent = header_str(headers, "user-agent");
    let origin = header_str(headers, "origin");

    info!(
        request_id = %request_id,
        protocol = %proto,
        accept = %accept,
        content_type = %content_type,
        session_id = %session_id,
        user_agent = %user_agent,
        origin = %origin,
        "Incoming request headers (summary)"
    );

    // Detailed header log at debug level with redaction and truncation
    let detailed: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| {
            let name_str = name.as_str().to_string();
            let lower = name_str.to_ascii_lowercase();
            let mut val = match value.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => "<non-utf8>".to_string(),
            };
            if SENSITIVE_HEADERS.contains(&lower.as_str()) {
                val = "<redacted>".to_string();
            } else if val.len() > 256 {
                val = format!("{}…", &val[..256]);
            }
            (name_str, val)
        })
        .collect();

    debug!(request_id = %request_id, headers = ?detailed, "Incoming request headers (detailed)");
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

        self.sessions
            .write()
            .map_err(|_| TransportError::SessionLockError)?
            .insert(session_id, session);

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
        sessions.get(&session_id).map_or(
            Err(TransportError::SessionNotFound(session_id)),
            |session| {
                session.update_activity();
                Ok(())
            },
        )
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
        drop(sessions);
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
            Self::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed"),
            Self::UnsupportedProtocolVersion(_) => {
                (StatusCode::BAD_REQUEST, "Unsupported Protocol Version")
            }
            Self::SessionNotFound(_) => (StatusCode::BAD_REQUEST, "Session Not Found"),
            Self::InvalidSessionId(_) => (StatusCode::BAD_REQUEST, "Invalid Session ID"),
            Self::SessionLockError => (StatusCode::INTERNAL_SERVER_ERROR, "Session Lock Error"),
            Self::MissingContentType => (StatusCode::BAD_REQUEST, "Missing Content-Type"),
            Self::InvalidContentType(_) => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type")
            }
            Self::JsonParseError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON"),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large"),
            Self::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
            Self::SecurityValidationFailed(_) => {
                (StatusCode::FORBIDDEN, "Security Validation Failed")
            }
            Self::InvalidAcceptHeader(_) => (StatusCode::BAD_REQUEST, "Invalid Accept Header"),
            Self::UnacceptableAcceptHeader(_) => (StatusCode::NOT_ACCEPTABLE, "Not Acceptable"),
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

/// Unified MCP endpoint handler supporting both POST (JSON) and GET (SSE)
///
/// This handler processes all MCP requests according to the 2025-06-18 specification:
/// - POST requests with application/json -> JSON-RPC processing
/// - GET requests with text/event-stream -> SSE for Streamable HTTP transport
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
    // Generate unique request ID for tracing
    let request_id = Uuid::new_v4();

    // Extract protocol version for logging (clone to avoid borrow issues)
    let protocol_version = headers
        .get("MCP-Protocol-Version")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("missing")
        .to_string();

    let method = request.method().clone();
    let uri = request.uri().clone();

    // Create a span for structured logging with request context
    let span = tracing::info_span!(
        "mcp_request",
        request_id = %request_id,
        method = %method,
        uri = %uri,
        protocol_version = %protocol_version
    );

    async move {
        // Increment total request counter (count every incoming request)
        metrics().increment_requests();

        // Log request headers for compatibility/debugging
        log_request_headers(request_id, &headers);

        // Enhanced logging for Cursor debugging
        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let is_cursor = user_agent.to_lowercase().contains("cursor");

        if is_cursor {
            let accept_header = headers
                .get("accept")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let is_sse_get = method == Method::GET
                && (accept_header.contains("text/event-stream")
                    || accept_header.contains("text/*")
                    || accept_header.contains("*/*"));
            info!(
                "🔍 CURSOR REQUEST DETECTED: {} {} (protocol: {})",
                method, uri, protocol_version
            );
            // Avoid duplicate header dumps for SSE GET; detailed SSE headers are logged in handle_sse_request
            if !is_sse_get {
                for (name, value) in &headers {
                    if let Ok(v) = value.to_str() {
                        info!("  Header: {}: {}", name, v);
                    }
                }
            }
        } else {
            info!(
                "Processing MCP request: {} {} (protocol: {})",
                method, uri, protocol_version
            );
        }

        unified_mcp_handler_impl(state, headers, request, request_id).await
    }
    .instrument(span)
    .await
}

/// Internal implementation of the MCP handler with request ID context
async fn unified_mcp_handler_impl(
    state: McpServerState,
    headers: HeaderMap,
    request: Request<Body>,
    request_id: Uuid,
) -> Result<Response, TransportError> {
    // Validate protocol version first
    if let Err(status) = validate_protocol_version(&headers) {
        metrics().increment_protocol_version_errors();
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

    // Validate Accept header for method compatibility
    // POST: application/json, GET: text/event-stream
    validate_accept_header(&headers, request.method())?;

    match *request.method() {
        Method::POST => handle_json_rpc_request(state, headers, request, request_id).await,
        Method::DELETE => handle_delete_session_request(&state, &headers, request_id),
        Method::GET => {
            // Gate SSE behind an env flag for compatibility with acceptance tests
            if std::env::var("MCP_ENABLE_SSE")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            {
                handle_sse_request(&state, &headers, request_id)
            } else {
                warn!(request_id = %request_id, "GET /mcp not enabled (MCP_ENABLE_SSE not set) - returning 405");
                Err(TransportError::MethodNotAllowed)
            }
        }
        _ => {
            metrics().increment_method_not_allowed();
            warn!(request_id = %request_id, method = %request.method(), "Unsupported HTTP method");
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
                    // Allow both JSON responses and streaming via SSE for Streamable HTTP
                    if accept_header.contains("application/json")
                        || accept_header.contains("application/*")
                        || accept_header.contains("text/event-stream")
                        || accept_header.contains("text/*")
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
                    // For SSE requests, Accept should be compatible with text/event-stream
                    if accept_header.contains("text/event-stream")
                        || accept_header.contains("*/*")
                        || accept_header.contains("text/*")
                    {
                        Ok(())
                    } else {
                        warn!("Unacceptable Accept header for SSE: {accept_header}");
                        Err(TransportError::UnacceptableAcceptHeader(
                            accept_header.to_string(),
                        ))
                    }
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
    // First, try to extract session ID from headers (standard approach)
    if let Some(session_header) = headers.get(MCP_SESSION_ID) {
        if let Ok(session_str) = session_header.to_str() {
            if let Ok(session_id) = Uuid::parse_str(session_str) {
                // Check if session exists in comprehensive session manager
                if let Ok(session) = state.comprehensive_session_manager.get_session(session_id) {
                    if session.is_expired() {
                        debug!("Comprehensive session expired: {} (age: {:?}, idle: {:?}), will create new session",
                            session_id, session.age(), session.idle_time());
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
                        debug!(
                            "Reusing existing comprehensive session: {} (age: {:?}, idle: {:?})",
                            session_id,
                            session.age(),
                            session.idle_time()
                        );
                        return Ok(session_id);
                    }
                } else {
                    debug!("Session {} not found in comprehensive session manager, will create new session", session_id);
                }
            }
        }
    }

    // Fallback: For clients that don't preserve session IDs (like Cursor),
    // try to use a client identifier to maintain session continuity
    if let Some(ref info) = client_info {
        // Check for MCP_CLIENT_ID environment variable or X-Client-Id header
        let client_id = std::env::var("MCP_CLIENT_ID").ok().or_else(|| {
            headers
                .get("X-Client-Id")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        });

        if let Some(client_id) = client_id {
            // Generate a deterministic session ID based on client identifier
            // This allows the same client to reconnect to the same session
            let stable_session_id = generate_stable_session_id(&client_id, info);

            // Try to get existing session with this ID
            if let Ok(session) = state
                .comprehensive_session_manager
                .get_session(stable_session_id)
            {
                if !session.is_expired() {
                    let _ = state
                        .comprehensive_session_manager
                        .update_last_accessed(stable_session_id);
                    debug!(
                        "Reusing client-based session: {} for client_id: {} (age: {:?}, idle: {:?})",
                        stable_session_id,
                        client_id,
                        session.age(),
                        session.idle_time()
                    );
                    return Ok(stable_session_id);
                }
                debug!(
                    "Client-based session expired for client_id: {}, creating new",
                    client_id
                );
            }

            // Create new session with stable ID
            let session_id = state
                .comprehensive_session_manager
                .create_session_with_id(stable_session_id, client_info.clone())
                .map_err(|e| {
                    TransportError::InternalError(format!("Session creation failed: {e}"))
                })?;

            debug!(session_id = %session_id, client_id = %client_id, "Created new client-based session");
            metrics().increment_sessions_created();
            return Ok(session_id);
        }
    }

    // Standard path: Create new session with random ID
    let session_id = state
        .comprehensive_session_manager
        .create_session(client_info)
        .map_err(|e| TransportError::InternalError(format!("Session creation failed: {e}")))?;

    metrics().increment_sessions_created();
    debug!(session_id = %session_id, "Created new comprehensive session");
    Ok(session_id)
}

/// Generate a stable session ID based on client identifier
#[allow(clippy::cast_possible_truncation)]
fn generate_stable_session_id(client_id: &str, client_info: &ClientInfo) -> Uuid {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    client_id.hash(&mut hasher);

    // Include user agent for additional uniqueness
    if let Some(ref user_agent) = client_info.user_agent {
        user_agent.hash(&mut hasher);
    }

    // Create a deterministic UUID from the hash
    // Note: Truncation is intentional here - we're extracting bytes from the hash
    let hash = hasher.finish();
    let bytes = [
        (hash >> 56) as u8,
        (hash >> 48) as u8,
        (hash >> 40) as u8,
        (hash >> 32) as u8,
        (hash >> 24) as u8,
        (hash >> 16) as u8,
        (hash >> 8) as u8,
        hash as u8,
        0x40, // Version 4
        0x80, // Variant
        0,
        0,
        0,
        0,
        0,
        0, // Padding
    ];

    Uuid::from_bytes(bytes)
}

/// Handle DELETE requests for explicit session termination
fn handle_delete_session_request(
    state: &McpServerState,
    headers: &HeaderMap,
    request_id: Uuid,
) -> Result<Response, TransportError> {
    debug!(request_id = %request_id, "Processing DELETE session request");

    // Security validation first
    if let Err(e) = validate_origin(headers, &state.security_config) {
        error!(request_id = %request_id, "Origin validation failed for DELETE: {}", e);
        return Err(TransportError::SecurityValidationFailed(e.to_string()));
    }

    if let Err(e) = validate_dns_rebinding(headers, &state.security_config) {
        error!(request_id = %request_id, "DNS rebinding validation failed for DELETE: {}", e);
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
                    metrics().increment_sessions_deleted();
                    debug!(request_id = %request_id, session_id = %session_id, "Successfully deleted session");

                    // Create response with proper headers
                    let mut response_headers = HeaderMap::new();
                    set_standard_headers(&mut response_headers, Some(session_id));
                    add_security_headers(&mut response_headers);

                    Ok((StatusCode::NO_CONTENT, response_headers, "").into_response())
                } else {
                    debug!(request_id = %request_id, session_id = %session_id, "Session not found for deletion");

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
        warn!("Missing session ID in DELETE request - treating as unsupported method");
        Err(TransportError::MethodNotAllowed)
    }
}

/// Handle JSON-RPC requests over HTTP POST
#[allow(clippy::too_many_lines)]
async fn handle_json_rpc_request(
    state: McpServerState,
    headers: HeaderMap,
    request: Request<Body>,
    request_id: Uuid,
) -> Result<Response, TransportError> {
    debug!(request_id = %request_id, "Processing JSON-RPC request");

    // Security validation first
    if let Err(e) = validate_origin(&headers, &state.security_config) {
        metrics().increment_security_validation_errors();
        error!(request_id = %request_id, "Origin validation failed: {}", e);
        return Err(TransportError::SecurityValidationFailed(e.to_string()));
    }

    if let Err(e) = validate_dns_rebinding(&headers, &state.security_config) {
        metrics().increment_security_validation_errors();
        error!(request_id = %request_id, "DNS rebinding validation failed: {}", e);
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
    // Note: Session creation metrics are tracked inside get_or_create_comprehensive_session

    debug!(request_id = %request_id, session_id = %session_id, "Session associated with request");

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

    // Log raw body for Cursor debugging
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if user_agent.to_lowercase().contains("cursor") {
        info!(request_id = %request_id, "🔍 CURSOR RAW REQUEST BODY: {}", String::from_utf8_lossy(&body_bytes));
    }

    // Parse JSON-RPC request
    let json_request: Value = serde_json::from_slice(&body_bytes).map_err(|e| {
        metrics().increment_json_parse_errors();
        TransportError::JsonParseError(e.to_string())
    })?;

    // Extract JSON-RPC id (if present) for observability
    let jsonrpc_id_str = json_request
        .get("id")
        .map_or_else(|| "-".to_string(), ToString::to_string);

    // Enhanced logging for Cursor
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if user_agent.to_lowercase().contains("cursor") {
        info!(
            request_id = %request_id,
            session_id = %session_id,
            jsonrpc_id = %jsonrpc_id_str,
            "🔍 CURSOR PARSED JSON-RPC: {}",
            serde_json::to_string_pretty(&json_request).unwrap_or_else(|_| json_request.to_string())
        );
    } else {
        debug!(
            request_id = %request_id,
            session_id = %session_id,
            jsonrpc_id = %jsonrpc_id_str,
            "Parsed JSON-RPC request: {}",
            json_request
        );
    }

    // Process through existing MCP handler
    let jsonrpc_id_value = json_request.get("id").cloned().unwrap_or(Value::Null);
    let accept_header = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let wants_sse_stream = accept_header.contains("text/event-stream")
        || accept_header.contains("text/*");

    match state.handler.handle_request(json_request).await {
        Ok(result_value) => {
            metrics().increment_post_success();
            // Enhanced logging for Cursor responses
            let user_agent = headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if user_agent.to_lowercase().contains("cursor") {
                info!(request_id = %request_id, session_id = %session_id, "🔍 CURSOR RESPONSE: {}", 
                    serde_json::to_string_pretty(&result_value).unwrap_or_else(|_| result_value.to_string()));
            } else {
                debug!(request_id = %request_id, session_id = %session_id, "MCP handler result: {}", result_value);
            }

            // Update session activity using comprehensive session manager
            let _ = state
                .comprehensive_session_manager
                .update_last_accessed(session_id);

            // Wrap in JSON-RPC envelope
            let envelope = json!({
                "jsonrpc": "2.0",
                "id": jsonrpc_id_value,
                "result": result_value
            });

            // Publish to session SSE hub for Streamable-HTTP clients (always)
            let payload = serde_json::to_string(&envelope)
                .unwrap_or_else(|_| envelope.to_string());
            let _ = SSE_HUB.publish(session_id, SseMessage { id: None, event: Some("message".to_string()), data: payload });

            if wants_sse_stream {
                // Acknowledge with 200 + headers to confirm receipt
                let mut response_headers = HeaderMap::new();
                set_json_response_headers(&mut response_headers, Some(session_id));
                add_security_headers(&mut response_headers);
                Ok((StatusCode::OK, response_headers, Json(json!({"status":"streaming"}))).into_response())
            } else {
                // Standard JSON response
                let mut response_headers = HeaderMap::new();
                set_json_response_headers(&mut response_headers, Some(session_id));
                add_security_headers(&mut response_headers);
                Ok((StatusCode::OK, response_headers, Json(envelope)).into_response())
            }
        }
        Err(e) => {
            metrics().increment_internal_errors();

            // Enhanced error logging for Cursor
            let user_agent = headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if user_agent.to_lowercase().contains("cursor") {
                error!(request_id = %request_id, session_id = %session_id, "🔍 CURSOR REQUEST FAILED: {}", e);
            } else {
                error!(request_id = %request_id, session_id = %session_id, "MCP handler failed: {}", e);
            }

            let error_envelope = json!({
                "jsonrpc": "2.0",
                "id": jsonrpc_id_value,
                "error": {
                    "code": -32603,
                    "message": "Internal Server Error",
                    "data": format!("Handler error: {e}")
                }
            });

            if user_agent.to_lowercase().contains("cursor") {
                info!(request_id = %request_id, "🔍 CURSOR ERROR RESPONSE: {}", 
                    serde_json::to_string_pretty(&error_envelope).unwrap_or_else(|_| error_envelope.to_string()));
            }

            // Publish error to SSE hub as well (always)
            let payload = serde_json::to_string(&error_envelope)
                .unwrap_or_else(|_| error_envelope.to_string());
            let _ = SSE_HUB.publish(session_id, SseMessage { id: None, event: Some("error".to_string()), data: payload });

            if wants_sse_stream {
                let mut response_headers = HeaderMap::new();
                set_json_response_headers(&mut response_headers, Some(session_id));
                add_security_headers(&mut response_headers);
                Ok((StatusCode::OK, response_headers, Json(json!({"status":"streaming"}))).into_response())
            } else {
                // Return JSON-RPC error envelope with HTTP 200 to follow JSON-RPC semantics
                let mut response_headers = HeaderMap::new();
                set_json_response_headers(&mut response_headers, Some(session_id));
                add_security_headers(&mut response_headers);
                Ok((StatusCode::OK, response_headers, Json(error_envelope)).into_response())
            }
        }
    }
}

/// Handle SSE connection for Streamable HTTP transport
///
/// # Errors
/// Returns a `TransportError` if session creation or SSE setup fails.
fn handle_sse_request(
    state: &McpServerState,
    headers: &HeaderMap,
    request_id: Uuid,
) -> Result<Response, TransportError> {
    // Enhanced logging for SSE requests from Cursor
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if user_agent.to_lowercase().contains("cursor") {
        info!(request_id = %request_id, "🔍 CURSOR SSE REQUEST - Establishing SSE connection");
        // Log ALL headers at INFO level for Cursor SSE
        for (name, value) in headers {
            if let Ok(v) = value.to_str() {
                info!("  SSE Header: {}: {}", name, v);
            }
        }
    } else {
        info!(request_id = %request_id, "Establishing SSE connection for MCP Streamable HTTP transport");
    }

    // Get or create session (include client info so MCP_CLIENT_ID/X-Client-Id can be honored)
    let client_info = extract_client_info(headers);
    let session_id = get_or_create_comprehensive_session(state, headers, Some(client_info))?;

    // Note: Do not increment POST success metrics here; GET establishes SSE only

    // Build a streaming SSE response that stays open until the client disconnects.
    // Include explicit capabilities per MCP spec expectations.
    let init_payload = format!(
        "{{\"jsonrpc\": \"2.0\", \"method\": \"notifications/initialized\", \"params\": {{\"protocolVersion\": \"{SUPPORTED_PROTOCOL_VERSION}\", \"capabilities\": {{\"resources\": {{}}, \"prompts\": {{}}, \"tools\": {{}}, \"sampling\": {{}}, \"roots\": {{}}, \"elicitation\": {{}}}}, \"serverInfo\": {{\"name\": \"mcp\", \"version\": \"{}\"}}}}}}",
        env!("CARGO_PKG_VERSION")
    );

    let mut interval = tokio::time::interval(Duration::from_secs(15));
    let last_event_id = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let mut rx = SSE_HUB.subscribe(session_id);
    let replay = SSE_HUB.snapshot_from(session_id, last_event_id);

    let stream = async_stream::stream! {
        // 1) Send initialization event (id: 0)
        let init_event = Event::default()
            .id("0")
            .event("initialized")
            .data(init_payload.clone())
            .retry(Duration::from_millis(3000));
        yield Ok::<Event, Infallible>(init_event);

        // 2) Replay buffered events after Last-Event-ID
        for (id, msg) in replay {
            let mut ev = Event::default();
            if let Some(ev_name) = msg.event.clone() { ev = ev.event(ev_name); }
            ev = ev.id(id.to_string());
            ev = ev.data(msg.data.clone());
            yield Ok::<Event, Infallible>(ev);
        }

        // 3) Live loop: keep-alives + new messages
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let keep = Event::default().comment("keep-alive");
                    yield Ok::<Event, Infallible>(keep);
                }
                recv = rx.recv() => {
                    match recv {
                        Ok(msg) => {
                            let mut ev = Event::default();
                            if let Some(ev_name) = msg.event.clone() { ev = ev.event(ev_name); }
                            if let Some(id) = msg.id.clone() { ev = ev.id(id); }
                            ev = ev.data(msg.data.clone());
                            yield Ok::<Event, Infallible>(ev);
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Receiver lagged behind; send a hint comment
                            let warn_ev = Event::default().comment("lagged: events dropped");
                            yield Ok::<Event, Infallible>(warn_ev);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            }
        }
    };

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text(": keep-alive"),
    );

    let mut response = sse.into_response();
    // Set protocol/session/security headers explicitly for clients
    let headers_mut = response.headers_mut();
    crate::headers::set_sse_response_headers(headers_mut, Some(session_id));
    add_security_headers(headers_mut);

    debug!(request_id = %request_id, "Started persistent SSE stream");
    Ok(response)
}

/// Initialize transport with session cleanup task
///
/// This function starts a background task that periodically cleans up expired sessions.
/// It should be called during server startup.
pub async fn initialize_transport(session_manager: SessionManager) {
    let cleanup_interval = Duration::from_secs(60); // Cleanup every minute
    let manager = session_manager;

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

//! Security validation module for MCP server
//!
//! This module provides comprehensive security features including Origin header validation,
//! DNS rebinding protection, and localhost binding enforcement for secure local deployments.

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashSet;
use std::net::IpAddr;
use thiserror::Error;
use tracing::{debug, error, warn};

/// Security configuration for the MCP server
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Set of allowed origins for CORS and DNS rebinding protection
    pub allowed_origins: HashSet<String>,
    /// Enable strict origin validation (reject requests without valid origins)
    pub strict_origin_validation: bool,
    /// Restrict server binding to localhost only for security
    pub localhost_only: bool,
    /// Require Origin header on all requests (recommended for web security)
    pub require_origin_header: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("http://localhost:3001".to_string());
        allowed_origins.insert("https://localhost:3001".to_string());
        allowed_origins.insert("http://127.0.0.1:3001".to_string());
        allowed_origins.insert("https://127.0.0.1:3001".to_string());
        allowed_origins.insert("http://[::1]:3001".to_string());
        allowed_origins.insert("https://[::1]:3001".to_string());

        Self {
            allowed_origins,
            strict_origin_validation: true,
            localhost_only: true,
            require_origin_header: false, // Keep flexible for MVP
        }
    }
}

impl SecurityConfig {
    /// Create a new security configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from environment variables with sensible defaults.
    ///
    /// Supported env vars:
    /// - `MCP_ALLOWED_ORIGINS` (comma-separated)
    /// - `MCP_STRICT_ORIGIN_VALIDATION` (true/false)
    /// - `MCP_REQUIRE_ORIGIN_HEADER` (true/false)
    /// - `MCP_LOCALHOST_ONLY` (true/false)
    #[must_use]
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        // Allowed origins
        if let Ok(list) = std::env::var("MCP_ALLOWED_ORIGINS") {
            let mut set: HashSet<String> = HashSet::new();
            for item in list.split(',') {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    set.insert(trimmed.to_string());
                }
            }
            if !set.is_empty() {
                cfg.allowed_origins = set;
            }
        }

        // Booleans
        if let Ok(v) = std::env::var("MCP_STRICT_ORIGIN_VALIDATION") {
            cfg.strict_origin_validation = matches!(v.as_str(), "1" | "true" | "TRUE" | "True");
        }
        if let Ok(v) = std::env::var("MCP_REQUIRE_ORIGIN_HEADER") {
            cfg.require_origin_header = matches!(v.as_str(), "1" | "true" | "TRUE" | "True");
        }
        if let Ok(v) = std::env::var("MCP_LOCALHOST_ONLY") {
            cfg.localhost_only = matches!(v.as_str(), "1" | "true" | "TRUE" | "True");
        }

        cfg
    }

    /// Add an allowed origin to the configuration
    pub fn add_allowed_origin(&mut self, origin: &str) -> &mut Self {
        self.allowed_origins.insert(origin.to_string());
        self
    }

    /// Set strict origin validation mode
    #[must_use]
    pub const fn with_strict_origin_validation(mut self, strict: bool) -> Self {
        self.strict_origin_validation = strict;
        self
    }

    /// Set localhost-only binding mode
    #[must_use]
    pub const fn with_localhost_only(mut self, localhost_only: bool) -> Self {
        self.localhost_only = localhost_only;
        self
    }

    /// Set whether Origin header is required
    #[must_use]
    pub const fn with_require_origin_header(mut self, require: bool) -> Self {
        self.require_origin_header = require;
        self
    }

    /// Validate a given origin against the allowed origins list
    #[must_use]
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.contains(origin)
    }

    /// Check if an origin represents a localhost variant
    #[must_use]
    pub fn is_localhost_origin(&self, origin: &str) -> bool {
        origin.contains("localhost") || origin.contains("127.0.0.1") || origin.contains("[::1]")
    }
}

/// Security validation errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Origin not allowed: {0}")]
    OriginNotAllowed(String),

    #[error("Missing required Origin header")]
    MissingOriginHeader,

    #[error("DNS rebinding attack detected - Host: {host}, Origin: {origin}")]
    DnsRebindingDetected { host: String, origin: String },

    #[error("Invalid origin format: {0}")]
    InvalidOriginFormat(String),

    #[error("Localhost binding required but server not bound to localhost")]
    LocalhostBindingRequired,

    #[error("Invalid host header: {0}")]
    InvalidHostHeader(String),
}

impl IntoResponse for SecurityError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::OriginNotAllowed(_) => (StatusCode::FORBIDDEN, "Origin not allowed"),
            Self::MissingOriginHeader => (StatusCode::BAD_REQUEST, "Missing Origin header"),
            Self::DnsRebindingDetected { .. } => {
                (StatusCode::FORBIDDEN, "DNS rebinding attack detected")
            }
            Self::InvalidOriginFormat(_) => (StatusCode::BAD_REQUEST, "Invalid origin format"),
            Self::LocalhostBindingRequired => (StatusCode::FORBIDDEN, "Localhost binding required"),
            Self::InvalidHostHeader(_) => (StatusCode::BAD_REQUEST, "Invalid Host header"),
        };

        error!("Security validation error: {}", self);

        let error_response = json!({
            "error": {
                "code": -32600,
                "message": error_message,
                "data": self.to_string()
            }
        });

        (status, Json(error_response)).into_response()
    }
}

/// Extract origin from request headers
fn extract_origin(headers: &HeaderMap) -> Option<String> {
    headers
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .map(String::from)
}

/// Extract host from request headers
fn extract_host(headers: &HeaderMap) -> Option<String> {
    headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(String::from)
}

/// Validate origin header against security configuration
///
/// # Errors
///
/// Returns `SecurityError` if origin validation fails
pub fn validate_origin(headers: &HeaderMap, config: &SecurityConfig) -> Result<(), SecurityError> {
    let origin = extract_origin(headers);

    // Check if Origin header is required
    if config.require_origin_header && origin.is_none() {
        return Err(SecurityError::MissingOriginHeader);
    }

    // If origin is present, validate it
    if let Some(origin_value) = origin {
        // When strict validation is enabled, enforce scheme + allow‑list
        if config.strict_origin_validation {
            if !origin_value.starts_with("http://") && !origin_value.starts_with("https://") {
                return Err(SecurityError::InvalidOriginFormat(origin_value));
            }
            if !config.is_origin_allowed(&origin_value) {
                warn!("Origin not allowed: {}", origin_value);
                return Err(SecurityError::OriginNotAllowed(origin_value));
            }
        } // else: be permissive for non-browser/native clients (Cursor often sends `Origin: null`)
    }

    Ok(())
}

/// Validate Host header against DNS rebinding attacks
///
/// # Errors
///
/// Returns `SecurityError` if DNS rebinding attack is detected
pub fn validate_dns_rebinding(
    headers: &HeaderMap,
    config: &SecurityConfig,
) -> Result<(), SecurityError> {
    let host = extract_host(headers);
    let origin = extract_origin(headers);

    // If both headers are present, validate they match for security
    if let (Some(host_value), Some(origin_value)) = (host, origin) {
        // Parse origin to extract host part
        let origin_host = url::Url::parse(&origin_value).map_or(None, |url| {
            url.host_str().map(|h| {
                url.port()
                    .map_or_else(|| h.to_string(), |port| format!("{h}:{port}"))
            })
        });

        // Check for DNS rebinding attack
        if let Some(origin_host_value) = origin_host {
            // Allow localhost variants
            let is_safe = config.is_localhost_origin(&host_value)
                && config.is_localhost_origin(&origin_host_value);

            if !is_safe && host_value != origin_host_value {
                error!(
                    "DNS rebinding attack detected - Host: {}, Origin: {}",
                    host_value, origin_value
                );
                return Err(SecurityError::DnsRebindingDetected {
                    host: host_value,
                    origin: origin_value,
                });
            }
        }
    }

    Ok(())
}

/// Origin validation middleware for Axum
///
/// # Errors
///
/// Returns `SecurityError` if origin validation or DNS rebinding protection fails.
pub async fn origin_validation_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, SecurityError> {
    // Extract security config from request extensions or use default
    // In a real implementation, this would come from server state
    let config = SecurityConfig::default();

    // Validate origin header
    validate_origin(&headers, &config)?;

    // Validate against DNS rebinding attacks
    validate_dns_rebinding(&headers, &config)?;

    debug!("Security validation passed for request: {}", request.uri());

    // Continue with request processing
    let response = next.run(request).await;
    Ok(response)
}

/// Validate server binding address for localhost-only mode
///
/// # Errors
///
/// Returns `SecurityError::LocalhostBindingRequired` if server is not bound to localhost
pub fn validate_server_binding(
    bind_addr: &str,
    config: &SecurityConfig,
) -> Result<(), SecurityError> {
    if !config.localhost_only {
        return Ok(());
    }

    // Parse the bind address - handle both IPv4:port and [IPv6]:port formats
    let addr_str = if bind_addr.starts_with('[') {
        // IPv6 format: [::1]:3001
        bind_addr
            .split(']')
            .next()
            .and_then(|s| s.strip_prefix('['))
            .unwrap_or(bind_addr)
    } else if bind_addr.contains(':') {
        // IPv4 format: 127.0.0.1:3001 or IPv6 without brackets: ::1:3001
        let parts: Vec<&str> = bind_addr.split(':').collect();
        if parts.len() > 2 {
            // Likely IPv6 without brackets, take all but last part
            &bind_addr[..bind_addr.rfind(':').unwrap_or(bind_addr.len())]
        } else {
            // IPv4:port format
            parts[0]
        }
    } else {
        bind_addr
    };

    // Check if address is localhost
    match addr_str {
        "127.0.0.1" | "localhost" | "::1" | "[::1]" => Ok(()),
        "0.0.0.0" | "::" => {
            error!(
                "Server binding to {} is not secure for localhost-only mode",
                bind_addr
            );
            Err(SecurityError::LocalhostBindingRequired)
        }
        _ => {
            // Try to parse as IP address
            addr_str.parse::<IpAddr>().map_or_else(
                |_| {
                    error!("Invalid server bind address: {}", bind_addr);
                    Err(SecurityError::LocalhostBindingRequired)
                },
                |ip| {
                    if ip.is_loopback() {
                        Ok(())
                    } else {
                        error!("Server binding to {} is not localhost", bind_addr);
                        Err(SecurityError::LocalhostBindingRequired)
                    }
                },
            )
        }
    }
}

/// Add security headers to response
pub fn add_security_headers(headers: &mut HeaderMap) {
    // Add security headers for enhanced protection
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
}

/// Security audit logging for monitoring
pub fn log_security_event(event_type: &str, details: &str, severity: SecurityEventSeverity) {
    match severity {
        SecurityEventSeverity::Info => debug!("Security event [{}]: {}", event_type, details),
        SecurityEventSeverity::Warning => warn!("Security event [{}]: {}", event_type, details),
        SecurityEventSeverity::Critical => error!("Security event [{}]: {}", event_type, details),
    }
}

/// Severity levels for security events
#[derive(Debug, Clone, Copy)]
pub enum SecurityEventSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_headers(origin: Option<&str>, host: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(o) = origin {
            headers.insert("origin", HeaderValue::from_str(o).unwrap());
        }
        if let Some(h) = host {
            headers.insert("host", HeaderValue::from_str(h).unwrap());
        }
        headers
    }

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.strict_origin_validation);
        assert!(config.localhost_only);
        assert!(!config.require_origin_header);
        assert!(config.is_origin_allowed("http://localhost:3001"));
        assert!(config.is_origin_allowed("https://127.0.0.1:3001"));
    }

    #[test]
    fn test_security_config_builder() {
        let mut config = SecurityConfig::new()
            .with_strict_origin_validation(false)
            .with_localhost_only(false)
            .with_require_origin_header(true);

        config.add_allowed_origin("https://example.com");

        assert!(!config.strict_origin_validation);
        assert!(!config.localhost_only);
        assert!(config.require_origin_header);
        assert!(config.is_origin_allowed("https://example.com"));
    }

    #[test]
    fn test_localhost_origin_detection() {
        let config = SecurityConfig::default();

        assert!(config.is_localhost_origin("http://localhost:3001"));
        assert!(config.is_localhost_origin("https://127.0.0.1:8080"));
        assert!(config.is_localhost_origin("http://[::1]:3001"));
        assert!(!config.is_localhost_origin("https://example.com"));
    }

    #[test]
    fn test_origin_validation_success() {
        let config = SecurityConfig::default();
        let headers = create_test_headers(Some("http://localhost:3001"), None);

        assert!(validate_origin(&headers, &config).is_ok());
    }

    #[test]
    fn test_origin_validation_not_allowed() {
        let config = SecurityConfig::default();
        let headers = create_test_headers(Some("https://malicious.com"), None);

        assert!(matches!(
            validate_origin(&headers, &config),
            Err(SecurityError::OriginNotAllowed(_))
        ));
    }

    #[test]
    fn test_origin_validation_missing_required() {
        let config = SecurityConfig::default().with_require_origin_header(true);
        let headers = create_test_headers(None, None);

        assert!(matches!(
            validate_origin(&headers, &config),
            Err(SecurityError::MissingOriginHeader)
        ));
    }

    #[test]
    fn test_origin_validation_invalid_format() {
        let config = SecurityConfig::default();
        let headers = create_test_headers(Some("invalid-origin"), None);

        assert!(matches!(
            validate_origin(&headers, &config),
            Err(SecurityError::InvalidOriginFormat(_))
        ));
    }

    #[test]
    fn test_dns_rebinding_detection() {
        let config = SecurityConfig::default();
        let headers = create_test_headers(Some("https://malicious.com"), Some("localhost:3001"));

        // This should detect DNS rebinding attack
        assert!(matches!(
            validate_dns_rebinding(&headers, &config),
            Err(SecurityError::DnsRebindingDetected { .. })
        ));
    }

    #[test]
    fn test_dns_rebinding_localhost_allowed() {
        let config = SecurityConfig::default();
        let headers = create_test_headers(Some("http://localhost:3001"), Some("localhost:3001"));

        assert!(validate_dns_rebinding(&headers, &config).is_ok());
    }

    #[test]
    fn test_server_binding_validation() {
        let config = SecurityConfig::default();

        // Valid localhost bindings
        assert!(validate_server_binding("127.0.0.1:3001", &config).is_ok());
        assert!(validate_server_binding("localhost:3001", &config).is_ok());
        assert!(validate_server_binding("[::1]:3001", &config).is_ok());
        assert!(validate_server_binding("::1:3001", &config).is_ok()); // IPv6 without brackets (non-standard but supported)

        // Invalid bindings for localhost-only mode
        assert!(matches!(
            validate_server_binding("0.0.0.0:3001", &config),
            Err(SecurityError::LocalhostBindingRequired)
        ));
        assert!(matches!(
            validate_server_binding("192.168.1.100:3001", &config),
            Err(SecurityError::LocalhostBindingRequired)
        ));
    }

    #[test]
    fn test_server_binding_validation_disabled() {
        let config = SecurityConfig::default().with_localhost_only(false);

        // Should allow any binding when localhost_only is disabled
        assert!(validate_server_binding("0.0.0.0:3001", &config).is_ok());
        assert!(validate_server_binding("192.168.1.100:3001", &config).is_ok());
    }

    #[test]
    fn test_security_headers() {
        let mut headers = HeaderMap::new();
        add_security_headers(&mut headers);

        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("x-xss-protection"));
        assert!(headers.contains_key("referrer-policy"));
    }

    #[test]
    fn test_security_event_logging() {
        // This test mainly verifies the function compiles and runs
        log_security_event("test", "test event", SecurityEventSeverity::Info);
        log_security_event("warning", "test warning", SecurityEventSeverity::Warning);
        log_security_event("critical", "test critical", SecurityEventSeverity::Critical);
    }
}

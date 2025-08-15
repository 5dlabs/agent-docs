//! Session management and security unit tests
//!
//! These tests focus on testing the session and security modules in isolation
//! without requiring a full server setup or database connection.

use axum::http::{HeaderMap, HeaderValue};
use doc_server_mcp::{
    security::{
        validate_dns_rebinding, validate_origin, validate_server_binding, SecurityConfig,
        SecurityError,
    },
    session::{ClientInfo, SessionConfig, SessionError, SessionManager},
};
use std::time::Duration;
use uuid::Uuid;

/// Test session creation with secure UUID generation
#[test]
fn test_session_manager_creation() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    assert_eq!(manager.session_count().unwrap(), 0);
}

/// Test session creation and retrieval
#[test]
fn test_session_lifecycle() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    // Create a session
    let client_info = ClientInfo {
        user_agent: Some("TestClient/1.0".to_string()),
        origin: Some("http://localhost:3001".to_string()),
        ip_address: Some("127.0.0.1".to_string()),
    };

    let session_id = manager.create_session(Some(client_info)).unwrap();
    assert_eq!(manager.session_count().unwrap(), 1);

    // Retrieve the session
    let session = manager.get_session(session_id).unwrap();
    assert_eq!(session.session_id, session_id);
    assert!(!session.is_expired());
    assert_eq!(
        session.client_info.user_agent,
        Some("TestClient/1.0".to_string())
    );

    // Update session activity
    manager.update_last_accessed(session_id).unwrap();

    // Delete the session
    manager.delete_session(session_id).unwrap();
    assert_eq!(manager.session_count().unwrap(), 0);
}

/// Test session expiry logic
#[test]
fn test_session_expiry() {
    let config = SessionConfig {
        default_ttl: chrono::Duration::milliseconds(10),
        max_sessions: 100,
        cleanup_interval: chrono::Duration::minutes(1),
    };
    let manager = SessionManager::new(config);

    // Create a session with very short TTL
    let session_id = manager.create_session(None).unwrap();

    // Wait for expiry
    std::thread::sleep(Duration::from_millis(20));

    // Session should be expired but still in storage
    let session = manager.get_session(session_id);
    assert!(matches!(session, Err(SessionError::SessionExpired(_))));

    // Cleanup should remove expired session
    let cleaned = manager.cleanup_expired_sessions().unwrap();
    assert_eq!(cleaned, 1);
    assert_eq!(manager.session_count().unwrap(), 0);
}

/// Test session limit enforcement
#[test]
fn test_session_limit() {
    let config = SessionConfig {
        default_ttl: chrono::Duration::hours(1),
        max_sessions: 2,
        cleanup_interval: chrono::Duration::minutes(5),
    };
    let manager = SessionManager::new(config);

    // Create sessions up to limit
    let _session1 = manager.create_session(None).unwrap();
    let _session2 = manager.create_session(None).unwrap();
    assert_eq!(manager.session_count().unwrap(), 2);

    // Third session should fail
    let result = manager.create_session(None);
    assert!(matches!(result, Err(SessionError::MaxSessionsReached(2))));
}

/// Test session statistics
#[test]
fn test_session_statistics() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    // Empty statistics
    let stats = manager.session_stats().unwrap();
    assert_eq!(stats.total, 0);
    assert_eq!(stats.active, 0);
    assert_eq!(stats.expired, 0);

    // Create some sessions
    let _session1 = manager.create_session(None).unwrap();
    let _session2 = manager.create_session(None).unwrap();

    let stats = manager.session_stats().unwrap();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.active, 2);
    assert_eq!(stats.expired, 0);
}

/// Test security configuration defaults
#[test]
fn test_security_config_defaults() {
    let config = SecurityConfig::default();

    assert!(config.strict_origin_validation);
    assert!(config.localhost_only);
    assert!(!config.require_origin_header);

    // Check default allowed origins
    assert!(config.is_origin_allowed("http://localhost:3001"));
    assert!(config.is_origin_allowed("https://127.0.0.1:3001"));
    assert!(config.is_origin_allowed("http://[::1]:3001"));
    assert!(!config.is_origin_allowed("https://malicious.com"));
}

/// Test security configuration builder pattern
#[test]
fn test_security_config_builder() {
    let mut config = SecurityConfig::new()
        .with_strict_origin_validation(false)
        .with_localhost_only(false)
        .with_require_origin_header(true);

    config.add_allowed_origin("https://trusted.com");

    assert!(!config.strict_origin_validation);
    assert!(!config.localhost_only);
    assert!(config.require_origin_header);
    assert!(config.is_origin_allowed("https://trusted.com"));
}

/// Test localhost origin detection
#[test]
fn test_localhost_origin_detection() {
    let config = SecurityConfig::default();

    assert!(config.is_localhost_origin("http://localhost:3001"));
    assert!(config.is_localhost_origin("https://127.0.0.1:8080"));
    assert!(config.is_localhost_origin("http://[::1]:3001"));
    assert!(!config.is_localhost_origin("https://example.com"));
    assert!(!config.is_localhost_origin("https://192.168.1.100:3001"));
}

/// Helper to create header map
fn create_headers(origin: Option<&str>, host: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(o) = origin {
        headers.insert("origin", HeaderValue::from_str(o).unwrap());
    }
    if let Some(h) = host {
        headers.insert("host", HeaderValue::from_str(h).unwrap());
    }
    headers
}

/// Test origin validation with allowed origins
#[test]
fn test_origin_validation_allowed() {
    let config = SecurityConfig::default();
    let headers = create_headers(Some("http://localhost:3001"), None);

    let result = validate_origin(&headers, &config);
    assert!(result.is_ok());
}

/// Test origin validation with disallowed origins
#[test]
fn test_origin_validation_disallowed() {
    let config = SecurityConfig::default();
    let headers = create_headers(Some("https://malicious.com"), None);

    let result = validate_origin(&headers, &config);
    assert!(matches!(result, Err(SecurityError::OriginNotAllowed(_))));
}

/// Test origin validation with missing origin (when not required)
#[test]
fn test_origin_validation_missing_not_required() {
    let config = SecurityConfig::default().with_require_origin_header(false);
    let headers = create_headers(None, None);

    let result = validate_origin(&headers, &config);
    assert!(result.is_ok());
}

/// Test origin validation with missing origin (when required)
#[test]
fn test_origin_validation_missing_required() {
    let config = SecurityConfig::default().with_require_origin_header(true);
    let headers = create_headers(None, None);

    let result = validate_origin(&headers, &config);
    assert!(matches!(result, Err(SecurityError::MissingOriginHeader)));
}

/// Test origin validation with invalid format
#[test]
fn test_origin_validation_invalid_format() {
    let config = SecurityConfig::default();
    let headers = create_headers(Some("not-a-url"), None);

    let result = validate_origin(&headers, &config);
    assert!(matches!(result, Err(SecurityError::InvalidOriginFormat(_))));
}

/// Test DNS rebinding validation with safe localhost matching
#[test]
fn test_dns_rebinding_localhost_safe() {
    let config = SecurityConfig::default();
    let headers = create_headers(Some("http://localhost:3001"), Some("localhost:3001"));

    let result = validate_dns_rebinding(&headers, &config);
    assert!(result.is_ok());
}

/// Test DNS rebinding validation detects attack
#[test]
fn test_dns_rebinding_attack_detection() {
    let config = SecurityConfig::default();
    let headers = create_headers(Some("https://malicious.com"), Some("localhost:3001"));

    let result = validate_dns_rebinding(&headers, &config);
    assert!(matches!(
        result,
        Err(SecurityError::DnsRebindingDetected { .. })
    ));
}

/// Test server binding validation for localhost-only mode
#[test]
fn test_server_binding_localhost_validation() {
    let config = SecurityConfig::default(); // localhost_only = true

    // Valid localhost bindings
    assert!(validate_server_binding("127.0.0.1:3001", &config).is_ok());
    assert!(validate_server_binding("localhost:3001", &config).is_ok());
    assert!(validate_server_binding("::1:3001", &config).is_ok());

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

/// Test server binding validation when localhost-only is disabled
#[test]
fn test_server_binding_validation_disabled() {
    let config = SecurityConfig::default().with_localhost_only(false);

    // Should allow any binding when localhost_only is disabled
    assert!(validate_server_binding("0.0.0.0:3001", &config).is_ok());
    assert!(validate_server_binding("192.168.1.100:3001", &config).is_ok());
    assert!(validate_server_binding("127.0.0.1:3001", &config).is_ok());
}

/// Test concurrent session operations
#[tokio::test]
async fn test_concurrent_session_operations() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);
    let manager = std::sync::Arc::new(manager);

    // Create multiple concurrent tasks
    let mut handles = Vec::new();

    for i in 0..50 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let client_info = ClientInfo {
                user_agent: Some(format!("Client-{i}")),
                origin: Some("http://localhost:3001".to_string()),
                ip_address: Some("127.0.0.1".to_string()),
            };

            // Create session
            let session_id = manager_clone.create_session(Some(client_info)).unwrap();

            // Update activity
            manager_clone.update_last_accessed(session_id).unwrap();

            session_id
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let session_ids: Vec<Uuid> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All session IDs should be unique
    let unique_ids: std::collections::HashSet<_> = session_ids.iter().collect();
    assert_eq!(unique_ids.len(), 50);

    // Check session count
    assert_eq!(manager.session_count().unwrap(), 50);
}

/// Test session manager configuration
#[test]
fn test_session_manager_config() {
    let custom_config = SessionConfig {
        default_ttl: chrono::Duration::minutes(60),
        max_sessions: 500,
        cleanup_interval: chrono::Duration::minutes(10),
    };

    let manager = SessionManager::new(custom_config);
    let retrieved_config = manager.config();

    assert_eq!(retrieved_config.default_ttl, chrono::Duration::minutes(60));
    assert_eq!(retrieved_config.max_sessions, 500);
    assert_eq!(
        retrieved_config.cleanup_interval,
        chrono::Duration::minutes(10)
    );
}

/// Test client info default
#[test]
fn test_client_info_default() {
    let client_info = ClientInfo::default();

    assert!(client_info.user_agent.is_none());
    assert!(client_info.origin.is_none());
    assert!(client_info.ip_address.is_none());
}

/// Test client info with values
#[test]
fn test_client_info_with_values() {
    let client_info = ClientInfo {
        user_agent: Some("Mozilla/5.0".to_string()),
        origin: Some("https://localhost:3001".to_string()),
        ip_address: Some("127.0.0.1".to_string()),
    };

    assert_eq!(client_info.user_agent, Some("Mozilla/5.0".to_string()));
    assert_eq!(
        client_info.origin,
        Some("https://localhost:3001".to_string())
    );
    assert_eq!(client_info.ip_address, Some("127.0.0.1".to_string()));
}

/// Test that UUID v4 generation produces unique values
#[test]
fn test_uuid_uniqueness() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let mut session_ids = std::collections::HashSet::new();

    // Create 100 sessions and verify all UUIDs are unique
    for _ in 0..100 {
        let session_id = manager.create_session(None).unwrap();
        assert!(session_ids.insert(session_id), "Duplicate UUID generated");
    }

    assert_eq!(session_ids.len(), 100);
}

/// Test session manager handles invalid session operations gracefully
#[test]
fn test_invalid_session_operations() {
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);

    let random_uuid = Uuid::new_v4();

    // Getting non-existent session
    assert!(matches!(
        manager.get_session(random_uuid),
        Err(SessionError::SessionNotFound(_))
    ));

    // Updating non-existent session
    assert!(matches!(
        manager.update_last_accessed(random_uuid),
        Err(SessionError::SessionNotFound(_))
    ));

    // Deleting non-existent session
    assert!(matches!(
        manager.delete_session(random_uuid),
        Err(SessionError::SessionNotFound(_))
    ));
}

/// Test session refresh functionality
#[test]
fn test_session_refresh() {
    use chrono::Duration;
    use doc_server_mcp::session::Session;

    let ttl = Duration::minutes(30);
    let mut session = Session::new(ttl, None);

    let initial_access = session.last_accessed;

    // Wait a small amount and refresh
    std::thread::sleep(std::time::Duration::from_millis(10));
    session.refresh();

    assert!(session.last_accessed > initial_access);
    assert!(!session.is_expired());
    assert!(session.idle_time() < Duration::seconds(1));
}

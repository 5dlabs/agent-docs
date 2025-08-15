//! Session management for MCP server
//!
//! This module provides comprehensive session management including secure UUID v4 generation,
//! TTL support, client information tracking, protocol version consistency, and thread-safe operations.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::protocol_version::{ProtocolRegistry, SUPPORTED_PROTOCOL_VERSION};

/// Client information extracted from request headers for security and audit purposes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientInfo {
    /// User-Agent header for client identification
    pub user_agent: Option<String>,
    /// Origin header for security validation
    pub origin: Option<String>,
    /// IP address for audit logging (when available)
    pub ip_address: Option<String>,
}

/// MCP session with comprehensive lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Cryptographically secure session identifier (UUID v4)
    pub session_id: Uuid,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp for TTL calculation
    pub last_accessed: DateTime<Utc>,
    /// Session time-to-live duration
    pub ttl: Duration,
    /// Client information for security and audit
    pub client_info: ClientInfo,
    /// MCP protocol version for this session (fixed to 2025-06-18)
    pub protocol_version: String,
}

impl Session {
    /// Create a new session with secure UUID v4 generation
    #[must_use]
    pub fn new(ttl: Duration, client_info: Option<ClientInfo>) -> Self {
        let now = Utc::now();
        let registry = ProtocolRegistry::new();

        Self {
            session_id: Uuid::new_v4(),
            created_at: now,
            last_accessed: now,
            ttl,
            client_info: client_info.unwrap_or_default(),
            protocol_version: registry.current_version_string().to_string(),
        }
    }

    /// Create a new session with explicit protocol version
    #[must_use]
    pub fn new_with_version(
        ttl: Duration,
        client_info: Option<ClientInfo>,
        protocol_version: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4(),
            created_at: now,
            last_accessed: now,
            ttl,
            client_info: client_info.unwrap_or_default(),
            protocol_version,
        }
    }

    /// Check if session has expired based on TTL and last access time
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        now.signed_duration_since(self.last_accessed) > self.ttl
    }

    /// Update session activity timestamp (session renewal)
    pub fn refresh(&mut self) {
        self.last_accessed = Utc::now();
    }

    /// Get session age since creation
    #[must_use]
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        now.signed_duration_since(self.created_at)
    }

    /// Get time since last activity
    #[must_use]
    pub fn idle_time(&self) -> Duration {
        let now = Utc::now();
        now.signed_duration_since(self.last_accessed)
    }

    /// Check if the session's protocol version matches the supported version
    #[must_use]
    pub fn is_protocol_version_supported(&self) -> bool {
        self.protocol_version == SUPPORTED_PROTOCOL_VERSION
    }

    /// Validate that the protocol version matches the expected version
    ///
    /// # Errors
    ///
    /// Returns an error if the protocol version doesn't match the expected version.
    pub fn validate_protocol_version(&self, expected_version: &str) -> Result<(), SessionError> {
        if self.protocol_version == expected_version {
            Ok(())
        } else {
            Err(SessionError::ProtocolVersionMismatch {
                session_version: self.protocol_version.clone(),
                expected_version: expected_version.to_string(),
            })
        }
    }
}

/// Session management configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Default TTL for new sessions (30 minutes)
    pub default_ttl: Duration,
    /// Maximum number of concurrent sessions
    pub max_sessions: usize,
    /// Cleanup interval for expired sessions (5 minutes)
    pub cleanup_interval: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::minutes(30),
            max_sessions: 1000,
            cleanup_interval: Duration::minutes(5),
        }
    }
}

/// Session management errors
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Maximum sessions reached (limit: {0})")]
    MaxSessionsReached(usize),

    #[error("Lock acquisition failed")]
    LockError,

    #[error("Session expired: {0}")]
    SessionExpired(Uuid),

    #[error("Invalid session ID format: {0}")]
    InvalidSessionId(String),

    #[error(
        "Protocol version mismatch: session has {session_version}, expected {expected_version}"
    )]
    ProtocolVersionMismatch {
        session_version: String,
        expected_version: String,
    },
}

/// Thread-safe session manager with comprehensive lifecycle management
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// Thread-safe session storage
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    /// Session management configuration
    config: SessionConfig,
}

impl SessionManager {
    /// Create a new session manager with configuration
    #[must_use]
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new session with secure UUID v4 generation
    ///
    /// # Errors
    ///
    /// Returns `SessionError::MaxSessionsReached` if the session limit is exceeded.
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn create_session(&self, client_info: Option<ClientInfo>) -> Result<Uuid, SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;

        // Check session limit
        if sessions.len() >= self.config.max_sessions {
            warn!(
                "Session limit reached: {}/{}",
                sessions.len(),
                self.config.max_sessions
            );
            return Err(SessionError::MaxSessionsReached(self.config.max_sessions));
        }

        let session = Session::new(self.config.default_ttl, client_info);
        let session_id = session.session_id;

        sessions.insert(session_id, session);

        let registry = ProtocolRegistry::new();
        debug!(
            "Created new session: {} with protocol version {} (total: {})",
            session_id,
            registry.current_version_string(),
            sessions.len()
        );
        Ok(session_id)
    }

    /// Create a new session with explicit protocol version
    ///
    /// # Errors
    ///
    /// Returns `SessionError::MaxSessionsReached` if the session limit is exceeded.
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn create_session_with_version(
        &self,
        client_info: Option<ClientInfo>,
        protocol_version: &str,
    ) -> Result<Uuid, SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;

        // Check session limit
        if sessions.len() >= self.config.max_sessions {
            warn!(
                "Session limit reached: {}/{}",
                sessions.len(),
                self.config.max_sessions
            );
            return Err(SessionError::MaxSessionsReached(self.config.max_sessions));
        }

        let session = Session::new_with_version(
            self.config.default_ttl,
            client_info,
            protocol_version.to_string(),
        );
        let session_id = session.session_id;

        sessions.insert(session_id, session);

        debug!(
            "Created new session: {} with protocol version {} (total: {})",
            session_id,
            protocol_version,
            sessions.len()
        );
        Ok(session_id)
    }

    /// Get an existing session by ID
    ///
    /// # Errors
    ///
    /// Returns `SessionError::SessionNotFound` if the session doesn't exist.
    /// Returns `SessionError::SessionExpired` if the session has expired.
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn get_session(&self, session_id: Uuid) -> Result<Session, SessionError> {
        let sessions = self.sessions.read().map_err(|_| SessionError::LockError)?;

        sessions.get(&session_id).map_or_else(
            || {
                debug!("Session not found: {}", session_id);
                Err(SessionError::SessionNotFound(session_id))
            },
            |session| {
                if session.is_expired() {
                    debug!("Session expired: {}", session_id);
                    Err(SessionError::SessionExpired(session_id))
                } else {
                    Ok(session.clone())
                }
            },
        )
    }

    /// Update session activity timestamp (session renewal)
    ///
    /// # Errors
    ///
    /// Returns `SessionError::SessionNotFound` if the session doesn't exist.
    /// Returns `SessionError::SessionExpired` if the session has expired.
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn update_last_accessed(&self, session_id: Uuid) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;

        sessions.get_mut(&session_id).map_or_else(
            || {
                debug!("Cannot refresh non-existent session: {}", session_id);
                Err(SessionError::SessionNotFound(session_id))
            },
            |session| {
                if session.is_expired() {
                    debug!("Attempted to refresh expired session: {}", session_id);
                    Err(SessionError::SessionExpired(session_id))
                } else {
                    session.refresh();
                    debug!("Updated session activity: {}", session_id);
                    Ok(())
                }
            },
        )
    }

    /// Delete a session explicitly (for DELETE endpoint support)
    ///
    /// # Errors
    ///
    /// Returns `SessionError::SessionNotFound` if the session doesn't exist.
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn delete_session(&self, session_id: Uuid) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;

        if sessions.remove(&session_id).is_some() {
            debug!(
                "Deleted session: {} (total: {})",
                session_id,
                sessions.len()
            );
            Ok(())
        } else {
            debug!("Cannot delete non-existent session: {}", session_id);
            Err(SessionError::SessionNotFound(session_id))
        }
    }

    /// Clean up expired sessions and return the number removed
    ///
    /// # Errors
    ///
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;

        let initial_count = sessions.len();
        sessions.retain(|id, session| {
            if session.is_expired() {
                debug!("Cleaning up expired session: {}", id);
                false
            } else {
                true
            }
        });

        let cleaned_count = initial_count - sessions.len();
        if cleaned_count > 0 {
            debug!(
                "Cleaned up {} expired sessions (total: {})",
                cleaned_count,
                sessions.len()
            );
        }

        Ok(cleaned_count)
    }

    /// Get current session count for monitoring
    ///
    /// # Errors
    ///
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn session_count(&self) -> Result<usize, SessionError> {
        let sessions = self.sessions.read().map_err(|_| SessionError::LockError)?;
        Ok(sessions.len())
    }

    /// Validate protocol version for an existing session
    ///
    /// # Errors
    ///
    /// Returns `SessionError::SessionNotFound` if the session doesn't exist.
    /// Returns `SessionError::SessionExpired` if the session has expired.
    /// Returns `SessionError::ProtocolVersionMismatch` if the protocol version doesn't match.
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn validate_session_protocol_version(
        &self,
        session_id: Uuid,
        expected_version: &str,
    ) -> Result<(), SessionError> {
        let sessions = self.sessions.read().map_err(|_| SessionError::LockError)?;

        sessions.get(&session_id).map_or_else(
            || {
                debug!("Session not found: {session_id}");
                Err(SessionError::SessionNotFound(session_id))
            },
            |session| {
                if session.is_expired() {
                    debug!("Session expired: {session_id}");
                    Err(SessionError::SessionExpired(session_id))
                } else {
                    session.validate_protocol_version(expected_version)
                }
            },
        )
    }

    /// Get session statistics for monitoring
    ///
    /// # Errors
    ///
    /// Returns `SessionError::LockError` if the session storage cannot be accessed.
    pub fn session_stats(&self) -> Result<SessionStats, SessionError> {
        let sessions = self.sessions.read().map_err(|_| SessionError::LockError)?;

        let total = sessions.len();
        let mut expired = 0;
        let mut oldest_age = Duration::zero();
        let mut newest_age = Duration::MAX;

        for session in sessions.values() {
            if session.is_expired() {
                expired += 1;
            }

            let age = session.age();
            if age > oldest_age {
                oldest_age = age;
            }
            if age < newest_age {
                newest_age = age;
            }
        }

        // Handle empty session case
        if total == 0 {
            newest_age = Duration::zero();
        }
        drop(sessions);

        Ok(SessionStats {
            total,
            expired,
            active: total - expired,
            oldest_session_age: oldest_age,
            newest_session_age: newest_age,
        })
    }

    /// Start background cleanup task
    ///
    /// This function spawns a tokio task that periodically cleans up expired sessions.
    /// It should be called during server initialization.
    pub fn start_cleanup_task(&self) {
        let manager = self.clone();
        let cleanup_interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                cleanup_interval.num_seconds().max(1).unsigned_abs(),
            ));

            loop {
                interval.tick().await;

                match manager.cleanup_expired_sessions() {
                    Ok(cleaned) => {
                        if cleaned > 0 {
                            debug!(
                                "Background session cleanup: removed {} expired sessions",
                                cleaned
                            );
                        }
                    }
                    Err(e) => {
                        error!("Background session cleanup failed: {}", e);
                    }
                }
            }
        });

        debug!(
            "Started background session cleanup task (interval: {:?})",
            cleanup_interval
        );
    }

    /// Get session configuration
    #[must_use]
    pub const fn config(&self) -> &SessionConfig {
        &self.config
    }
}

/// Session statistics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    /// Total number of sessions in storage
    pub total: usize,
    /// Number of expired sessions
    pub expired: usize,
    /// Number of active (non-expired) sessions
    pub active: usize,
    /// Age of the oldest session
    pub oldest_session_age: Duration,
    /// Age of the newest session
    pub newest_session_age: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;
    use tokio::time::sleep;

    #[test]
    fn test_session_creation() {
        let client_info = ClientInfo {
            user_agent: Some("test-client/1.0".to_string()),
            origin: Some("http://localhost:3001".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
        };

        let session = Session::new(Duration::minutes(30), Some(client_info));

        assert!(!session.is_expired());
        assert_eq!(
            session.client_info.user_agent,
            Some("test-client/1.0".to_string())
        );
        assert_eq!(
            session.client_info.origin,
            Some("http://localhost:3001".to_string())
        );
        assert!(session.age() < Duration::seconds(1));
    }

    #[test]
    fn test_session_expiry() {
        let mut session = Session::new(Duration::milliseconds(1), None);

        // Wait for expiry
        std::thread::sleep(StdDuration::from_millis(2));
        assert!(session.is_expired());

        // Test refresh
        session.refresh();
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_manager_creation() {
        let config = SessionConfig::default();
        let manager = SessionManager::new(config);

        assert_eq!(manager.session_count().unwrap(), 0);
    }

    #[test]
    fn test_session_lifecycle() {
        let config = SessionConfig::default();
        let manager = SessionManager::new(config);

        // Create session
        let session_id = manager.create_session(None).unwrap();
        assert_eq!(manager.session_count().unwrap(), 1);

        // Get session
        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.session_id, session_id);

        // Update activity
        manager.update_last_accessed(session_id).unwrap();

        // Delete session
        manager.delete_session(session_id).unwrap();
        assert_eq!(manager.session_count().unwrap(), 0);

        // Verify deletion
        assert!(matches!(
            manager.get_session(session_id),
            Err(SessionError::SessionNotFound(_))
        ));
    }

    #[test]
    fn test_session_limit() {
        let config = SessionConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        // Create sessions up to limit
        let _session1 = manager.create_session(None).unwrap();
        let _session2 = manager.create_session(None).unwrap();

        // Should fail to create third session
        assert!(matches!(
            manager.create_session(None),
            Err(SessionError::MaxSessionsReached(2))
        ));
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let config = SessionConfig {
            default_ttl: Duration::milliseconds(10),
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        // Create sessions
        let _session1 = manager.create_session(None).unwrap();
        let _session2 = manager.create_session(None).unwrap();
        assert_eq!(manager.session_count().unwrap(), 2);

        // Wait for expiry
        sleep(StdDuration::from_millis(15)).await;

        // Cleanup expired sessions
        let cleaned = manager.cleanup_expired_sessions().unwrap();
        assert_eq!(cleaned, 2);
        assert_eq!(manager.session_count().unwrap(), 0);
    }

    #[test]
    fn test_session_stats() {
        let config = SessionConfig::default();
        let manager = SessionManager::new(config);

        // Empty stats
        let stats = manager.session_stats().unwrap();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.active, 0);

        // Create sessions
        let _session1 = manager.create_session(None).unwrap();
        let _session2 = manager.create_session(None).unwrap();

        let stats = manager.session_stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.active, 2);
        assert_eq!(stats.expired, 0);
    }
}

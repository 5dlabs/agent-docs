//! Basic metrics collection for MCP server
//!
//! This module provides simple counters for tracking requests and errors.
//! For MVP, we use atomic counters. In production, these could be extended
//! to integrate with Prometheus or other metrics systems.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

/// Global metrics collection
pub struct McpMetrics {
    /// Total number of requests received
    pub requests_total: AtomicU64,
    /// Total number of successful POST requests
    pub post_requests_success: AtomicU64,
    /// Total number of 405 Method Not Allowed responses
    pub method_not_allowed_total: AtomicU64,
    /// Total number of protocol version errors
    pub protocol_version_errors: AtomicU64,
    /// Total number of JSON parsing errors
    pub json_parse_errors: AtomicU64,
    /// Total number of security validation errors
    pub security_validation_errors: AtomicU64,
    /// Total number of internal errors
    pub internal_errors: AtomicU64,
    /// Total number of sessions created
    pub sessions_created: AtomicU64,
    /// Total number of sessions deleted
    pub sessions_deleted: AtomicU64,
}

impl McpMetrics {
    /// Create a new metrics collection
    #[must_use]
    pub const fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            post_requests_success: AtomicU64::new(0),
            method_not_allowed_total: AtomicU64::new(0),
            protocol_version_errors: AtomicU64::new(0),
            json_parse_errors: AtomicU64::new(0),
            security_validation_errors: AtomicU64::new(0),
            internal_errors: AtomicU64::new(0),
            sessions_created: AtomicU64::new(0),
            sessions_deleted: AtomicU64::new(0),
        }
    }

    /// Increment total requests counter
    pub fn increment_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment successful POST requests counter
    pub fn increment_post_success(&self) {
        self.post_requests_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment method not allowed counter
    pub fn increment_method_not_allowed(&self) {
        self.method_not_allowed_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment protocol version errors counter
    pub fn increment_protocol_version_errors(&self) {
        self.protocol_version_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment JSON parse errors counter
    pub fn increment_json_parse_errors(&self) {
        self.json_parse_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment security validation errors counter
    pub fn increment_security_validation_errors(&self) {
        self.security_validation_errors
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment internal errors counter
    pub fn increment_internal_errors(&self) {
        self.internal_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment sessions created counter
    pub fn increment_sessions_created(&self) {
        self.sessions_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment sessions deleted counter
    pub fn increment_sessions_deleted(&self) {
        self.sessions_deleted.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics as a snapshot
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            post_requests_success: self.post_requests_success.load(Ordering::Relaxed),
            method_not_allowed_total: self.method_not_allowed_total.load(Ordering::Relaxed),
            protocol_version_errors: self.protocol_version_errors.load(Ordering::Relaxed),
            json_parse_errors: self.json_parse_errors.load(Ordering::Relaxed),
            security_validation_errors: self.security_validation_errors.load(Ordering::Relaxed),
            internal_errors: self.internal_errors.load(Ordering::Relaxed),
            sessions_created: self.sessions_created.load(Ordering::Relaxed),
            sessions_deleted: self.sessions_deleted.load(Ordering::Relaxed),
        }
    }
}

impl Default for McpMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of current metrics values
#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub requests_total: u64,
    pub post_requests_success: u64,
    pub method_not_allowed_total: u64,
    pub protocol_version_errors: u64,
    pub json_parse_errors: u64,
    pub security_validation_errors: u64,
    pub internal_errors: u64,
    pub sessions_created: u64,
    pub sessions_deleted: u64,
}

/// Global metrics instance
pub static METRICS: LazyLock<McpMetrics> = LazyLock::new(McpMetrics::new);

/// Convenience function to get global metrics instance
#[must_use]
pub fn metrics() -> &'static McpMetrics {
    &METRICS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = McpMetrics::new();
        let snapshot = metrics.snapshot();

        // All counters should start at zero
        assert_eq!(snapshot.requests_total, 0);
        assert_eq!(snapshot.post_requests_success, 0);
        assert_eq!(snapshot.method_not_allowed_total, 0);
        assert_eq!(snapshot.protocol_version_errors, 0);
        assert_eq!(snapshot.json_parse_errors, 0);
        assert_eq!(snapshot.security_validation_errors, 0);
        assert_eq!(snapshot.internal_errors, 0);
        assert_eq!(snapshot.sessions_created, 0);
        assert_eq!(snapshot.sessions_deleted, 0);
    }

    #[test]
    fn test_metrics_increment() {
        let metrics = McpMetrics::new();

        metrics.increment_requests();
        metrics.increment_post_success();
        metrics.increment_method_not_allowed();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 1);
        assert_eq!(snapshot.post_requests_success, 1);
        assert_eq!(snapshot.method_not_allowed_total, 1);
    }

    #[test]
    fn test_global_metrics() {
        let metrics1 = metrics();
        let metrics2 = metrics();

        // Should be the same instance
        assert!(std::ptr::eq(metrics1, metrics2));
    }
}

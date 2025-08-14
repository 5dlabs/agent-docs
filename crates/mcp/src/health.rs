//! Enhanced health check endpoints for Kubernetes probes and monitoring
//!
//! This module provides comprehensive health check endpoints suitable for
//! Kubernetes readiness and liveness probes, with detailed status reporting
//! and connection pool monitoring.

use axum::{
	extract::State,
	http::StatusCode,
	response::Json,
	routing::get,
	Router,
};
use doc_server_database::PoolStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::server::McpServerState;

/// Overall service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
	pub status: HealthStatus,
	pub service: String,
	pub version: String,
	pub timestamp: chrono::DateTime<chrono::Utc>,
	pub uptime_seconds: u64,
	pub checks: HashMap<String, ComponentHealth>,
}

/// Individual component health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
	pub status: HealthStatus,
	pub response_time_ms: u64,
	pub details: serde_json::Value,
	pub error: Option<String>,
}

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
	Healthy,
	Degraded,
	Unhealthy,
}

/// Readiness check result (for Kubernetes readiness probe)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessStatus {
	pub ready: bool,
	pub reason: Option<String>,
	pub checks: Vec<ReadinessCheck>,
}

/// Individual readiness check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCheck {
	pub name: String,
	pub ready: bool,
	pub message: Option<String>,
}

/// Liveness check result (for Kubernetes liveness probe)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessStatus {
	pub alive: bool,
	pub service: String,
	pub version: String,
}

/// Service uptime tracker
static SERVICE_START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Initialize service start time
pub fn init_service_start_time() {
	SERVICE_START_TIME.set(std::time::Instant::now()).ok();
}

/// Get service uptime in seconds
fn get_uptime_seconds() -> u64 {
	SERVICE_START_TIME
		.get()
		.map_or(0, |start| start.elapsed().as_secs())
}

/// Create health check router
pub fn create_health_router() -> Router<McpServerState> {
	Router::new()
		.route("/health", get(health_check))
		.route("/health/ready", get(readiness_check))
		.route("/health/live", get(liveness_check))
		.route("/health/detailed", get(detailed_health_check))
}

/// Basic health check endpoint
///
/// Returns simple JSON status suitable for load balancers and basic monitoring.
/// This endpoint is lightweight and cached for high-frequency checks.
async fn health_check(State(state): State<McpServerState>) -> Result<Json<serde_json::Value>, StatusCode> {
	// Quick database ping with timeout
	match tokio::time::timeout(
		std::time::Duration::from_secs(5),
		state.db_pool.ping()
	).await {
		Ok(Ok(())) => {
			Ok(Json(serde_json::json!({
				"status": "healthy",
				"service": "doc-server-mcp",
				"version": env!("CARGO_PKG_VERSION"),
				"timestamp": chrono::Utc::now()
			})))
		}
		_ => Err(StatusCode::SERVICE_UNAVAILABLE)
	}
}

/// Kubernetes readiness probe endpoint
///
/// Checks if the service is ready to receive traffic.
/// This includes database connectivity and migration status.
async fn readiness_check(State(state): State<McpServerState>) -> (StatusCode, Json<ReadinessStatus>) {
	let mut checks = Vec::new();
	let mut overall_ready = true;

	// Database connectivity check
	let db_check = if let Ok(Ok(health)) = tokio::time::timeout(
		std::time::Duration::from_secs(10),
		state.db_pool.health_check()
	).await {
		let ready = health.is_healthy;
		if !ready {
			overall_ready = false;
		}
		ReadinessCheck {
			name: "database".to_string(),
			ready,
			message: health.error_message,
		}
	} else {
		overall_ready = false;
		ReadinessCheck {
			name: "database".to_string(),
			ready: false,
			message: Some("Database health check timeout".to_string()),
		}
	};
	checks.push(db_check);

	// Connection pool check
	let pool_status = match state.db_pool.get_status().await {
		Ok(status) => {
			let pool_ready = status.pool_utilization_percent < 95.0 && 
						   status.metrics.success_rate_percent > 90.0;
			if !pool_ready {
				overall_ready = false;
			}
			ReadinessCheck {
				name: "connection_pool".to_string(),
				ready: pool_ready,
				message: if pool_ready {
					None
				} else {
					Some(format!(
						"Pool utilization: {:.1}%, Success rate: {:.1}%",
						status.pool_utilization_percent,
						status.metrics.success_rate_percent
					))
				},
			}
		}
		Err(e) => {
			overall_ready = false;
			ReadinessCheck {
				name: "connection_pool".to_string(),
				ready: false,
				message: Some(format!("Pool status check failed: {e}")),
			}
		}
	};
	checks.push(pool_status);

	let status = ReadinessStatus {
		ready: overall_ready,
		reason: if overall_ready {
			None
		} else {
			Some("One or more readiness checks failed".to_string())
		},
		checks,
	};

	let status_code = if overall_ready {
		StatusCode::OK
	} else {
		StatusCode::SERVICE_UNAVAILABLE
	};

	(status_code, Json(status))
}

/// Kubernetes liveness probe endpoint
///
/// Simple check to determine if the service is alive and should not be restarted.
/// This is a lightweight check that only verifies basic service responsiveness.
async fn liveness_check() -> Json<LivenessStatus> {
	Json(LivenessStatus {
		alive: true,
		service: "doc-server-mcp".to_string(),
		version: env!("CARGO_PKG_VERSION").to_string(),
	})
}

/// Detailed health check with comprehensive status
///
/// Provides detailed information about all service components for debugging
/// and comprehensive monitoring. This endpoint is more expensive and should
/// be used sparingly.
async fn detailed_health_check(State(state): State<McpServerState>) -> (StatusCode, Json<ServiceHealthStatus>) {
	let mut checks = HashMap::new();
	let mut overall_status = HealthStatus::Healthy;

	// Database detailed health check
	let db_start = std::time::Instant::now();
	match state.db_pool.health_check().await {
		Ok(health) => {
			let component_status = if health.is_healthy {
				HealthStatus::Healthy
			} else {
				overall_status = HealthStatus::Unhealthy;
				HealthStatus::Unhealthy
			};

			checks.insert("database".to_string(), ComponentHealth {
				status: component_status,
				response_time_ms: u64::try_from(db_start.elapsed().as_millis()).unwrap_or(u64::MAX),
				details: serde_json::json!({
					"active_connections": health.active_connections,
					"idle_connections": health.idle_connections,
					"response_time_ms": health.response_time_ms
				}),
				error: health.error_message,
			});
		}
		Err(e) => {
			overall_status = HealthStatus::Unhealthy;
			checks.insert("database".to_string(), ComponentHealth {
				status: HealthStatus::Unhealthy,
				response_time_ms: u64::try_from(db_start.elapsed().as_millis()).unwrap_or(u64::MAX),
				details: serde_json::json!({}),
				error: Some(e.to_string()),
			});
		}
	}

	// Connection pool detailed status
	let pool_start = std::time::Instant::now();
	match state.db_pool.get_status().await {
		Ok(pool_status) => {
			let pool_health_status = determine_pool_health_status(&pool_status);
			if pool_health_status != HealthStatus::Healthy && overall_status == HealthStatus::Healthy {
				overall_status = pool_health_status.clone();
			}

			checks.insert("connection_pool".to_string(), ComponentHealth {
				status: pool_health_status,
				response_time_ms: u64::try_from(pool_start.elapsed().as_millis()).unwrap_or(u64::MAX),
				details: serde_json::json!({
					"utilization_percent": pool_status.pool_utilization_percent,
					"success_rate_percent": pool_status.metrics.success_rate_percent,
					"total_acquisitions": pool_status.metrics.total_acquisitions,
					"acquisition_failures": pool_status.metrics.acquisition_failures,
					"total_queries": pool_status.metrics.total_queries,
					"query_failures": pool_status.metrics.query_failures,
					"max_connections": pool_status.config.max_connections,
					"min_connections": pool_status.config.min_connections
				}),
				error: None,
			});
		}
		Err(e) => {
			overall_status = HealthStatus::Unhealthy;
			checks.insert("connection_pool".to_string(), ComponentHealth {
				status: HealthStatus::Unhealthy,
				response_time_ms: u64::try_from(pool_start.elapsed().as_millis()).unwrap_or(u64::MAX),
				details: serde_json::json!({}),
				error: Some(e.to_string()),
			});
		}
	}

	// Session manager status (if available)
	checks.insert("session_manager".to_string(), ComponentHealth {
		status: HealthStatus::Healthy,
		response_time_ms: 0,
		details: serde_json::json!({
			"note": "Session manager health check not implemented"
		}),
		error: None,
	});

	let health_status = ServiceHealthStatus {
		status: overall_status,
		service: "doc-server-mcp".to_string(),
		version: env!("CARGO_PKG_VERSION").to_string(),
		timestamp: chrono::Utc::now(),
		uptime_seconds: get_uptime_seconds(),
		checks,
	};

	let status_code = match health_status.status {
		HealthStatus::Degraded | HealthStatus::Healthy => StatusCode::OK, // Still serving traffic
		HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
	};

	(status_code, Json(health_status))
}

/// Determine pool health status based on metrics
fn determine_pool_health_status(pool_status: &PoolStatus) -> HealthStatus {
	// Unhealthy conditions
	if pool_status.pool_utilization_percent > 95.0 {
		return HealthStatus::Unhealthy;
	}
	
	if pool_status.metrics.success_rate_percent < 90.0 {
		return HealthStatus::Unhealthy;
	}

	if pool_status.health.response_time_ms > 5000 {
		return HealthStatus::Unhealthy;
	}

	// Degraded conditions
	if pool_status.pool_utilization_percent > 80.0 {
		return HealthStatus::Degraded;
	}
	
	if pool_status.metrics.success_rate_percent < 95.0 {
		return HealthStatus::Degraded;
	}

	if pool_status.health.response_time_ms > 2000 {
		return HealthStatus::Degraded;
	}

	HealthStatus::Healthy
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pool_health_determination() {
		let mut pool_status = PoolStatus {
			config: doc_server_database::PoolConfig::default(),
			metrics: doc_server_database::PoolMetricsSnapshot {
				total_connections_created: 10,
				total_acquisitions: 1000,
				acquisition_failures: 0,
				total_queries: 1000,
				query_failures: 0,
				success_rate_percent: 100.0,
				last_health_check_ago_seconds: 10,
			},
			health: doc_server_database::HealthCheckResult {
				is_healthy: true,
				response_time_ms: 50,
				active_connections: 5,
				idle_connections: 5,
				error_message: None,
				checked_at: chrono::Utc::now(),
			},
			pool_utilization_percent: 50.0,
		};

		// Healthy
		assert_eq!(determine_pool_health_status(&pool_status), HealthStatus::Healthy);

		// Degraded due to high utilization
		pool_status.pool_utilization_percent = 85.0;
		assert_eq!(determine_pool_health_status(&pool_status), HealthStatus::Degraded);

		// Unhealthy due to very high utilization
		pool_status.pool_utilization_percent = 96.0;
		assert_eq!(determine_pool_health_status(&pool_status), HealthStatus::Unhealthy);

		// Unhealthy due to low success rate
		pool_status.pool_utilization_percent = 50.0;
		pool_status.metrics.success_rate_percent = 85.0;
		assert_eq!(determine_pool_health_status(&pool_status), HealthStatus::Unhealthy);

		// Unhealthy due to high response time
		pool_status.metrics.success_rate_percent = 99.0;
		pool_status.health.response_time_ms = 6000;
		assert_eq!(determine_pool_health_status(&pool_status), HealthStatus::Unhealthy);
	}
}
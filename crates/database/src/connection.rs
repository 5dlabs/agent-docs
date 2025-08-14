//! Enhanced database connection management with production-grade features
//!
//! This module provides comprehensive database connection pool management with
//! retry logic, monitoring, health checks, and production-optimized configuration.

use crate::pool_config::PoolConfig;
use crate::retry::{RetryConfig, RetryExecutor};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Enhanced database connection pool with monitoring and health checks
#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
    config: PoolConfig,
    metrics: Arc<PoolMetrics>,
    health_cache: Arc<RwLock<HealthCheckCache>>,
}

/// Pool metrics for monitoring
#[derive(Debug, Default)]
pub struct PoolMetrics {
    /// Total connections created
    pub total_connections_created: AtomicU64,
    /// Total connection acquisition attempts
    pub total_acquisitions: AtomicU64,
    /// Total connection acquisition failures
    pub acquisition_failures: AtomicU64,
    /// Total query execution count
    pub total_queries: AtomicU64,
    /// Total query failures
    pub query_failures: AtomicU64,
    /// Last health check timestamp
    pub last_health_check: AtomicU64,
}

/// Health check cache to reduce database load
#[derive(Debug, Clone)]
struct HealthCheckCache {
    result: HealthCheckResult,
    cached_at: Instant,
    ttl: Duration,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub error_message: Option<String>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// Detailed pool status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    pub config: PoolConfig,
    pub metrics: PoolMetricsSnapshot,
    pub health: HealthCheckResult,
    pub pool_utilization_percent: f64,
}

/// Snapshot of pool metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetricsSnapshot {
    pub total_connections_created: u64,
    pub total_acquisitions: u64,
    pub acquisition_failures: u64,
    pub total_queries: u64,
    pub query_failures: u64,
    pub success_rate_percent: f64,
    pub last_health_check_ago_seconds: u64,
}

impl DatabasePool {
    /// Create a new database pool with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if a connection to the database cannot be established
    /// within the configured timeout or the connection URL is invalid.
    pub async fn new(database_url: &str) -> Result<Self> {
        let config = PoolConfig::builder().database_url(database_url).build()?;

        Self::with_config(config).await
    }

    /// Create a new database pool with environment-based configuration
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables are invalid or database connection fails.
    pub async fn from_env() -> Result<Self> {
        let config = PoolConfig::from_env()?;
        Self::with_config(config).await
    }

    /// Create a new database pool with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if a connection to the database cannot be established
    /// within the configured timeout or the configuration is invalid.
    pub async fn with_config(config: PoolConfig) -> Result<Self> {
        config.validate()?;
        config.log_summary();

        info!("Establishing database connection with retry logic...");

        let retry_config = RetryConfig::from_env();
        let retry_executor = RetryExecutor::with_config(retry_config);

        // Build connection string with application name
        let connection_url = if config.database_url.contains("application_name=") {
            config.database_url.clone()
        } else {
            format!(
                "{}{}application_name={}",
                config.database_url,
                if config.database_url.contains('?') {
                    "&"
                } else {
                    "?"
                },
                config.application_name
            )
        };

        let pool = retry_executor
            .execute_with_classification(|| async {
                let mut pool_options = PgPoolOptions::new()
                    .min_connections(config.min_connections)
                    .max_connections(config.max_connections)
                    .acquire_timeout(config.acquire_timeout())
                    .test_before_acquire(config.test_before_acquire);

                if let Some(max_lifetime) = config.max_lifetime() {
                    pool_options = pool_options.max_lifetime(max_lifetime);
                }

                if let Some(idle_timeout) = config.idle_timeout() {
                    pool_options = pool_options.idle_timeout(idle_timeout);
                }

                pool_options.connect(&connection_url).await
            })
            .await?;

        let metrics = Arc::new(PoolMetrics::default());
        let health_cache = Arc::new(RwLock::new(HealthCheckCache {
            result: HealthCheckResult {
                is_healthy: false,
                response_time_ms: 0,
                active_connections: 0,
                idle_connections: 0,
                error_message: None,
                checked_at: chrono::Utc::now(),
            },
            cached_at: Instant::now(),
            ttl: Duration::from_secs(5), // 5-second cache TTL
        }));

        let database_pool = Self {
            pool,
            config,
            metrics,
            health_cache,
        };

        // Perform initial health check
        let _ = database_pool.health_check().await;

        info!("Database connection pool established successfully");
        Ok(database_pool)
    }

    /// Get a reference to the underlying pool
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get pool configuration
    #[must_use]
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Test the database connection with metrics tracking
    ///
    /// # Errors
    ///
    /// Returns an error if the connectivity check query fails.
    pub async fn ping(&self) -> Result<()> {
        self.metrics.total_queries.fetch_add(1, Ordering::Relaxed);

        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => Ok(()),
            Err(e) => {
                self.metrics.query_failures.fetch_add(1, Ordering::Relaxed);
                Err(anyhow!("Database ping failed: {}", e))
            }
        }
    }

    /// Perform comprehensive health check with caching
    ///
    /// # Errors
    ///
    /// Returns an error if health check queries fail.
    pub async fn health_check(&self) -> Result<HealthCheckResult> {
        // Check cache first
        {
            let cache = self.health_cache.read().await;
            if cache.cached_at.elapsed() < cache.ttl {
                return Ok(cache.result.clone());
            }
        }

        // Perform fresh health check
        let start = Instant::now();
        let mut result = HealthCheckResult {
            is_healthy: true,
            response_time_ms: 0,
            active_connections: 0,
            idle_connections: 0,
            error_message: None,
            checked_at: chrono::Utc::now(),
        };

        // Test basic connectivity
        match self.ping().await {
            Ok(()) => {
                let elapsed_ms = start.elapsed().as_millis();
                result.response_time_ms = u64::try_from(elapsed_ms).unwrap_or(u64::MAX);

                // Get connection pool status
                let pool_size = self.get_pool_size();
                result.active_connections = pool_size.0;
                result.idle_connections = pool_size.1;
            }
            Err(e) => {
                result.is_healthy = false;
                result.error_message = Some(e.to_string());
            }
        }

        // Update cache
        {
            let mut cache = self.health_cache.write().await;
            cache.result = result.clone();
            cache.cached_at = Instant::now();
        }

        // Update metrics
        let now_sec = chrono::Utc::now().timestamp();
        let now_u64 = u64::try_from(now_sec).unwrap_or(0);
        self.metrics
            .last_health_check
            .store(now_u64, Ordering::Relaxed);

        Ok(result)
    }

    /// Get detailed pool status for monitoring
    ///
    /// # Errors
    ///
    /// Returns an error if status queries fail.
    pub async fn get_status(&self) -> Result<PoolStatus> {
        let health = self.health_check().await?;
        let metrics = self.get_metrics_snapshot();

        let pool_utilization = if self.config.max_connections > 0 {
            (f64::from(health.active_connections) / f64::from(self.config.max_connections)) * 100.0
        } else {
            0.0
        };

        Ok(PoolStatus {
            config: self.config.clone(),
            metrics,
            health,
            pool_utilization_percent: pool_utilization,
        })
    }

    /// Get current pool size (active, idle)
    fn get_pool_size(&self) -> (u32, u32) {
        // Note: sqlx doesn't expose pool metrics directly
        // This is a simplified implementation
        let active = self.pool.size();
        let idle = if active > 0 { active - 1 } else { 0 }; // Rough estimate
        (active, idle)
    }

    /// Get snapshot of current metrics
    #[must_use]
    pub fn get_metrics_snapshot(&self) -> PoolMetricsSnapshot {
        let total_acquisitions = self.metrics.total_acquisitions.load(Ordering::Relaxed);
        let acquisition_failures = self.metrics.acquisition_failures.load(Ordering::Relaxed);
        let total_queries = self.metrics.total_queries.load(Ordering::Relaxed);
        let query_failures = self.metrics.query_failures.load(Ordering::Relaxed);
        let last_health_check = self.metrics.last_health_check.load(Ordering::Relaxed);

        #[allow(clippy::cast_precision_loss)]
        let success_rate = if total_queries > 0 {
            ((total_queries - query_failures) as f64 / total_queries as f64) * 100.0
        } else {
            100.0
        };

        let last_check_ago = if last_health_check > 0 {
            let now_sec = chrono::Utc::now().timestamp();
            let now_u64 = u64::try_from(now_sec).unwrap_or(0);
            now_u64.saturating_sub(last_health_check)
        } else {
            0
        };

        PoolMetricsSnapshot {
            total_connections_created: self
                .metrics
                .total_connections_created
                .load(Ordering::Relaxed),
            total_acquisitions,
            acquisition_failures,
            total_queries,
            query_failures,
            success_rate_percent: success_rate,
            last_health_check_ago_seconds: last_check_ago,
        }
    }

    /// Start periodic pool status logging (runs in background)
    pub fn start_monitoring(&self) {
        let pool = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                match pool.get_status().await {
                    Ok(status) => {
                        if status.pool_utilization_percent > 80.0 {
                            warn!(
                                "High pool utilization: {:.1}% ({}/{} connections)",
                                status.pool_utilization_percent,
                                status.health.active_connections,
                                status.config.max_connections
                            );
                        }

                        if status.metrics.success_rate_percent < 95.0 {
                            warn!(
                                "Low query success rate: {:.1}% ({} failures / {} queries)",
                                status.metrics.success_rate_percent,
                                status.metrics.query_failures,
                                status.metrics.total_queries
                            );
                        }

                        info!(
                            "Pool status: {:.0}% utilization, {:.1}% success rate, {}ms avg response",
                            status.pool_utilization_percent,
                            status.metrics.success_rate_percent,
                            status.health.response_time_ms
                        );
                    }
                    Err(e) => {
                        error!("Failed to get pool status: {}", e);
                    }
                }
            }
        });
    }

    /// Execute a query with metrics tracking
    /// # Errors
    ///
    /// Returns any error produced by the provided `operation` closure.
    pub async fn execute_with_metrics<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&PgPool) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        self.metrics
            .total_acquisitions
            .fetch_add(1, Ordering::Relaxed);

        match operation(&self.pool).await {
            Ok(result) => Ok(result),
            Err(e) => {
                self.metrics
                    .acquisition_failures
                    .fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }
}

impl HealthCheckCache {}

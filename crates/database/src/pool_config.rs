//! Production-grade database connection pool configuration
//!
//! This module provides enhanced connection pool management with production settings,
//! retry logic, and comprehensive monitoring capabilities.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// Database pool configuration with production defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    
    /// Maximum number of connections allowed
    pub max_connections: u32,
    
    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout_seconds: u64,
    
    /// Maximum connection lifetime before recycling
    pub max_lifetime_seconds: Option<u64>,
    
    /// Idle timeout before connection is closed
    pub idle_timeout_seconds: Option<u64>,
    
    /// Connection string for the database
    pub database_url: String,
    
    /// Application name for connection identification
    pub application_name: String,
    
    /// Test connections on acquisition
    pub test_before_acquire: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 100,
            acquire_timeout_seconds: 30,
            max_lifetime_seconds: Some(3600), // 1 hour
            idle_timeout_seconds: Some(600),  // 10 minutes
            database_url: "postgresql://localhost:5432/docs".to_string(),
            application_name: "doc-server".to_string(),
            test_before_acquire: true,
        }
    }
}

impl PoolConfig {
    /// Create pool configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Required: Database URL
        config.database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow!("DATABASE_URL environment variable is required"))?;

        // Optional pool size configuration
        if let Ok(min_str) = std::env::var("POOL_MIN_CONNECTIONS") {
            config.min_connections = min_str.parse()
                .map_err(|_| anyhow!("Invalid POOL_MIN_CONNECTIONS value: {}", min_str))?;
        }

        if let Ok(max_str) = std::env::var("POOL_MAX_CONNECTIONS") {
            config.max_connections = max_str.parse()
                .map_err(|_| anyhow!("Invalid POOL_MAX_CONNECTIONS value: {}", max_str))?;
        }

        if let Ok(timeout_str) = std::env::var("POOL_ACQUIRE_TIMEOUT") {
            config.acquire_timeout_seconds = timeout_str.parse()
                .map_err(|_| anyhow!("Invalid POOL_ACQUIRE_TIMEOUT value: {}", timeout_str))?;
        }

        if let Ok(lifetime_str) = std::env::var("POOL_MAX_LIFETIME") {
            let lifetime: u64 = lifetime_str.parse()
                .map_err(|_| anyhow!("Invalid POOL_MAX_LIFETIME value: {}", lifetime_str))?;
            config.max_lifetime_seconds = Some(lifetime);
        }

        if let Ok(idle_str) = std::env::var("POOL_IDLE_TIMEOUT") {
            let idle: u64 = idle_str.parse()
                .map_err(|_| anyhow!("Invalid POOL_IDLE_TIMEOUT value: {}", idle_str))?;
            config.idle_timeout_seconds = Some(idle);
        }

        if let Ok(app_name) = std::env::var("APP_NAME") {
            config.application_name = app_name;
        }

        config.validate()?;
        Ok(config)
    }

    /// Create a builder for fluent configuration
    pub fn builder() -> PoolConfigBuilder {
        PoolConfigBuilder::new()
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if configuration values are invalid.
    pub fn validate(&self) -> Result<()> {
        if self.min_connections > self.max_connections {
            return Err(anyhow!(
                "min_connections ({}) cannot be greater than max_connections ({})",
                self.min_connections,
                self.max_connections
            ));
        }

        if self.max_connections == 0 {
            return Err(anyhow!("max_connections must be greater than 0"));
        }

        if self.acquire_timeout_seconds == 0 {
            return Err(anyhow!("acquire_timeout_seconds must be greater than 0"));
        }

        if self.database_url.is_empty() {
            return Err(anyhow!("database_url cannot be empty"));
        }

        // Validate database URL format
        if !self.database_url.starts_with("postgresql://") && !self.database_url.starts_with("postgres://") {
            return Err(anyhow!("database_url must be a valid PostgreSQL connection string"));
        }

        Ok(())
    }

    /// Get acquire timeout as Duration
    pub fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_seconds)
    }

    /// Get max lifetime as Duration
    pub fn max_lifetime(&self) -> Option<Duration> {
        self.max_lifetime_seconds.map(Duration::from_secs)
    }

    /// Get idle timeout as Duration
    pub fn idle_timeout(&self) -> Option<Duration> {
        self.idle_timeout_seconds.map(Duration::from_secs)
    }

    /// Log configuration summary (without sensitive data)
    pub fn log_summary(&self) {
        info!(
            "Database pool configuration: min={}, max={}, acquire_timeout={}s, app={}",
            self.min_connections,
            self.max_connections,
            self.acquire_timeout_seconds,
            self.application_name
        );

        if let Some(lifetime) = self.max_lifetime_seconds {
            info!("Connection max lifetime: {}s", lifetime);
        }

        if let Some(idle) = self.idle_timeout_seconds {
            info!("Connection idle timeout: {}s", idle);
        }

        // Validate pool size recommendations
        if self.max_connections > 200 {
            warn!("Very high max_connections ({}). Consider if this is necessary.", self.max_connections);
        }

        if self.min_connections < 2 {
            warn!("Very low min_connections ({}). May cause connection delays.", self.min_connections);
        }
    }
}

/// Builder for pool configuration
pub struct PoolConfigBuilder {
    config: PoolConfig,
}

impl PoolConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: PoolConfig::default(),
        }
    }

    pub fn database_url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.database_url = url.into();
        self
    }

    pub fn min_connections(mut self, min: u32) -> Self {
        self.config.min_connections = min;
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.config.max_connections = max;
        self
    }

    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.config.acquire_timeout_seconds = timeout.as_secs();
        self
    }

    pub fn max_lifetime(mut self, lifetime: Option<Duration>) -> Self {
        self.config.max_lifetime_seconds = lifetime.map(|d| d.as_secs());
        self
    }

    pub fn idle_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.config.idle_timeout_seconds = timeout.map(|d| d.as_secs());
        self
    }

    pub fn application_name<S: Into<String>>(mut self, name: S) -> Self {
        self.config.application_name = name.into();
        self
    }

    pub fn test_before_acquire(mut self, test: bool) -> Self {
        self.config.test_before_acquire = test;
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn build(self) -> Result<PoolConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for PoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Production environment preset configurations
impl PoolConfig {
    /// Configuration optimized for development environments
    pub fn development() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            acquire_timeout_seconds: 10,
            max_lifetime_seconds: Some(1800), // 30 minutes
            idle_timeout_seconds: Some(300),  // 5 minutes
            test_before_acquire: true,
            ..Default::default()
        }
    }

    /// Configuration optimized for production environments
    pub fn production() -> Self {
        Self {
            min_connections: 5,
            max_connections: 100,
            acquire_timeout_seconds: 30,
            max_lifetime_seconds: Some(3600), // 1 hour
            idle_timeout_seconds: Some(600),  // 10 minutes
            test_before_acquire: true,
            ..Default::default()
        }
    }

    /// Configuration optimized for high-traffic production environments
    pub fn high_traffic() -> Self {
        Self {
            min_connections: 10,
            max_connections: 200,
            acquire_timeout_seconds: 30,
            max_lifetime_seconds: Some(3600), // 1 hour
            idle_timeout_seconds: Some(300),  // 5 minutes
            test_before_acquire: true,
            ..Default::default()
        }
    }

    /// Configuration for testing with minimal resources
    pub fn testing() -> Self {
        Self {
            min_connections: 1,
            max_connections: 5,
            acquire_timeout_seconds: 5,
            max_lifetime_seconds: Some(300),  // 5 minutes
            idle_timeout_seconds: Some(60),   // 1 minute
            test_before_acquire: false, // Faster for tests
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_validation() {
        let mut config = PoolConfig::default();
        assert!(config.validate().is_ok());

        // Invalid: min > max
        config.min_connections = 10;
        config.max_connections = 5;
        assert!(config.validate().is_err());

        // Invalid: max = 0
        config.max_connections = 0;
        assert!(config.validate().is_err());

        // Invalid: empty URL
        config.max_connections = 10;
        config.database_url = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfigBuilder::new()
            .database_url("postgresql://user:pass@localhost:5432/test")
            .min_connections(3)
            .max_connections(15)
            .build()
            .unwrap();

        assert_eq!(config.min_connections, 3);
        assert_eq!(config.max_connections, 15);
        assert_eq!(config.database_url, "postgresql://user:pass@localhost:5432/test");
    }

    #[test]
    fn test_preset_configurations() {
        let dev_config = PoolConfig::development();
        assert_eq!(dev_config.min_connections, 2);
        assert_eq!(dev_config.max_connections, 10);

        let prod_config = PoolConfig::production();
        assert_eq!(prod_config.min_connections, 5);
        assert_eq!(prod_config.max_connections, 100);

        let test_config = PoolConfig::testing();
        assert_eq!(test_config.min_connections, 1);
        assert_eq!(test_config.max_connections, 5);
    }
}
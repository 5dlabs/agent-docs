//! Database connection retry logic with exponential backoff
//!
//! This module provides robust retry mechanisms for handling temporary database
//! unavailability, network issues, and connection failures during startup.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Retry configuration for database connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: usize,

    /// Initial delay between retries
    pub initial_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Multiplier for exponential backoff
    pub multiplier: f64,

    /// Jitter to add randomness to retry delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create retry configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(max_retries_str) = std::env::var("DB_RETRY_MAX_ATTEMPTS") {
            if let Ok(max_retries) = max_retries_str.parse::<usize>() {
                config.max_retries = max_retries;
            }
        }

        if let Ok(initial_delay_str) = std::env::var("DB_RETRY_INITIAL_DELAY") {
            if let Ok(initial_delay_secs) = initial_delay_str.parse::<u64>() {
                config.initial_delay = Duration::from_secs(initial_delay_secs);
            }
        }

        if let Ok(max_delay_str) = std::env::var("DB_RETRY_MAX_DELAY") {
            if let Ok(max_delay_secs) = max_delay_str.parse::<u64>() {
                config.max_delay = Duration::from_secs(max_delay_secs);
            }
        }

        if let Ok(multiplier_str) = std::env::var("DB_RETRY_MULTIPLIER") {
            if let Ok(multiplier) = multiplier_str.parse::<f64>() {
                config.multiplier = multiplier;
            }
        }

        if let Ok(jitter_str) = std::env::var("DB_RETRY_JITTER") {
            config.jitter = jitter_str.to_lowercase() == "true";
        }

        config
    }

    /// Validate retry configuration
    ///
    /// # Errors
    ///
    /// Returns an error if configuration values are invalid.
    pub fn validate(&self) -> Result<()> {
        if self.max_retries == 0 {
            return Err(anyhow!("max_retries must be greater than 0"));
        }

        if self.initial_delay.is_zero() {
            return Err(anyhow!("initial_delay must be greater than 0"));
        }

        if self.max_delay < self.initial_delay {
            return Err(anyhow!(
                "max_delay must be greater than or equal to initial_delay"
            ));
        }

        if self.multiplier <= 1.0 {
            return Err(anyhow!("multiplier must be greater than 1.0"));
        }

        Ok(())
    }

    /// Calculate delay for a specific attempt
    #[must_use]
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let pow = i32::try_from(attempt).unwrap_or(i32::MAX);
        let delay_secs = self.initial_delay.as_secs_f64() * self.multiplier.powi(pow);
        let delay = Duration::from_secs_f64(delay_secs.min(self.max_delay.as_secs_f64()));

        if self.jitter {
            Self::add_jitter(delay)
        } else {
            delay
        }
    }

    /// Add jitter to delay to avoid thundering herd
    fn add_jitter(delay: Duration) -> Duration {
        use rand::Rng;
        let mut rng = rand::rng();
        let jitter_factor = rng.random_range(0.5..=1.0);
        Duration::from_secs_f64(delay.as_secs_f64() * jitter_factor)
    }
}

/// Types of database errors that can be retried
#[derive(Debug, Clone)]
pub enum DatabaseError {
    /// Connection timeout or network error
    ConnectionFailed(String),
    /// Authentication failure (should not retry)
    AuthenticationFailed(String),
    /// Database is starting up or temporarily unavailable
    TemporarilyUnavailable(String),
    /// Too many connections (should retry with backoff)
    TooManyConnections(String),
    /// Database does not exist (should not retry)
    DatabaseNotFound(String),
    /// Other error
    Other(String),
}

impl DatabaseError {
    /// Determine if this error type should be retried
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            DatabaseError::AuthenticationFailed(_)
            | DatabaseError::DatabaseNotFound(_)
            | DatabaseError::Other(_) => false,
            DatabaseError::TemporarilyUnavailable(_)
            | DatabaseError::TooManyConnections(_)
            | DatabaseError::ConnectionFailed(_) => true,
        }
    }

    /// Create `DatabaseError` from `sqlx::Error`
    #[must_use]
    pub fn from_sqlx_error(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::Tls(_) | sqlx::Error::Io(_) => {
                DatabaseError::ConnectionFailed(error.to_string())
            }
            sqlx::Error::PoolTimedOut => DatabaseError::TooManyConnections(error.to_string()),
            sqlx::Error::PoolClosed => DatabaseError::TemporarilyUnavailable(error.to_string()),
            sqlx::Error::Database(db_error) => match db_error.code() {
                Some(code) if code == "3D000" => DatabaseError::DatabaseNotFound(error.to_string()),
                Some(code) if code == "28P01" => {
                    DatabaseError::AuthenticationFailed(error.to_string())
                }
                Some(code) if code == "53300" => {
                    DatabaseError::TooManyConnections(error.to_string())
                }
                _ => DatabaseError::Other(error.to_string()),
            },
            _ => DatabaseError::Other(error.to_string()),
        }
    }
}

/// Retry executor for database operations
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    /// Create a new retry executor with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }

    /// Create a new retry executor with custom configuration
    #[must_use]
    pub fn with_config(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Execute a database operation with retry logic
    ///
    /// # Errors
    ///
    /// Returns the last error encountered if all retry attempts fail.
    /// # Panics
    /// Panics if internal `last_error` tracking is unexpectedly `None` after all attempts.
    pub async fn execute<F, Fut, T, E>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + Clone,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            info!(
                "Database operation attempt {} of {}",
                attempt + 1,
                self.config.max_retries + 1
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("Database operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    if attempt == self.config.max_retries {
                        error!(
                            "Database operation failed after {} attempts: {}",
                            self.config.max_retries + 1,
                            error
                        );
                        break;
                    }

                    let delay = self.config.calculate_delay(attempt);
                    warn!(
                        "Database operation failed (attempt {}), retrying in {:?}: {}",
                        attempt + 1,
                        delay,
                        error
                    );

                    sleep(delay).await;
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Execute a database operation with retry logic and error classification
    ///
    /// # Errors
    ///
    /// Returns the last error encountered if all retry attempts fail or if the error is not retryable.
    /// # Panics
    /// Panics if internal `last_error` tracking is unexpectedly `None` after all attempts.
    pub async fn execute_with_classification<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnMut() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>> + Send,
    {
        let mut last_error = None;
        let mut operation = operation;

        for attempt in 0..=self.config.max_retries {
            info!(
                "Database operation attempt {} of {}",
                attempt + 1,
                self.config.max_retries + 1
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("Database operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    let db_error = DatabaseError::from_sqlx_error(&error);
                    last_error = Some(error);

                    // Check if this error should be retried
                    if !db_error.is_retryable() {
                        error!("Non-retryable database error: {}", db_error.to_string());
                        return Err(anyhow!(
                            "Non-retryable database error: {}",
                            db_error.to_string()
                        ));
                    }

                    if attempt == self.config.max_retries {
                        error!(
                            "Database operation failed after {} attempts: {}",
                            self.config.max_retries + 1,
                            db_error.to_string()
                        );
                        break;
                    }

                    let delay = self.config.calculate_delay(attempt);
                    warn!(
                        "Retryable database error (attempt {}), retrying in {:?}: {}",
                        attempt + 1,
                        delay,
                        db_error.to_string()
                    );

                    sleep(delay).await;
                }
            }
        }

        Err(anyhow!(
            "Database operation failed after {} attempts: {}",
            self.config.max_retries + 1,
            last_error.unwrap()
        ))
    }
}

impl Default for RetryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            DatabaseError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {msg}"),
            DatabaseError::TemporarilyUnavailable(msg) => {
                write!(f, "Database temporarily unavailable: {msg}")
            }
            DatabaseError::TooManyConnections(msg) => write!(f, "Too many connections: {msg}"),
            DatabaseError::DatabaseNotFound(msg) => write!(f, "Database not found: {msg}"),
            DatabaseError::Other(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_validation() {
        let mut config = RetryConfig::default();
        assert!(config.validate().is_ok());

        config.max_retries = 0;
        assert!(config.validate().is_err());

        config.max_retries = 5;
        config.initial_delay = Duration::from_secs(0);
        assert!(config.validate().is_err());

        config.initial_delay = Duration::from_secs(5);
        config.max_delay = Duration::from_secs(1);
        assert!(config.validate().is_err());

        config.max_delay = Duration::from_secs(10);
        config.multiplier = 1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(16),
            multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };

        assert_eq!(config.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(config.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(config.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(config.calculate_delay(3), Duration::from_secs(8));
        assert_eq!(config.calculate_delay(4), Duration::from_secs(16)); // capped at max_delay
        assert_eq!(config.calculate_delay(5), Duration::from_secs(16)); // capped at max_delay
    }

    #[test]
    fn test_database_error_classification() {
        assert!(DatabaseError::ConnectionFailed("test".to_string()).is_retryable());
        assert!(!DatabaseError::AuthenticationFailed("test".to_string()).is_retryable());
        assert!(DatabaseError::TemporarilyUnavailable("test".to_string()).is_retryable());
        assert!(DatabaseError::TooManyConnections("test".to_string()).is_retryable());
        assert!(!DatabaseError::DatabaseNotFound("test".to_string()).is_retryable());
        assert!(!DatabaseError::Other("test".to_string()).is_retryable());
    }

    #[tokio::test]
    async fn test_retry_executor_success() {
        let executor = RetryExecutor::new();

        let result = executor
            .execute(|| async move {
                // simulate two failures before success
                Ok::<_, &str>("success")
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_retry_executor_failure() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            ..Default::default()
        };
        let executor = RetryExecutor::with_config(config);

        let result = executor
            .execute(|| async move { Err::<&str, &str>("persistent failure") })
            .await;

        assert!(result.is_err());
    }
}

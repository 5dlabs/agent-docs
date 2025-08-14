# Task ID: 7
# Title: Enhanced Database Connection and Migration Validation (Extend Existing, Live DB Safe)
# Status: pending
# Dependencies: 1
# Priority: high
# Description: Establish production database connectivity, validate schema migration, and implement connection pooling with proper health checks.
# Details:
Configure SQLx connection pool with production PostgreSQL cluster credentials. Validate pgvector extension (3072 dimensions) availability. Verify documents and document_sources tables with proper constraints. Implement connection retry logic with exponential backoff. Add database health checks for Kubernetes probes. Configure pool size based on expected load (min: 5, max: 100). Use dotenvy for environment configuration. Implement proper connection timeout handling (30s default).

# Live DB Execution Policy:
- Operate against the live DB via `DATABASE_URL` with safeguards:
- Pre-migration backup and verified restore
- Staging dry-run using a fresh snapshot of production schema/data
- Zero-downtime migration strategy (additive changes, backfill)
- Maintenance window for any blocking steps
- Prefer roll-forward; document rollback only for emergencies
- Post-migration verification (integrity + performance smoke tests)

# Test Strategy:
Test connection establishment with production cluster, validate vector operations with 3072-dimension arrays, stress test connection pool under load, verify automatic reconnection after database restart, and validate migration state with schema comparisons.

# Subtasks:
## 1. Configure SQLx Connection Pool with Production Settings [pending]
### Dependencies: None
### Description: Enhance the existing DatabasePool implementation to support production-grade configuration with environment variables and configurable pool settings
### Details:
Update crates/database/src/connection.rs to read pool configuration from environment variables (POOL_MIN_CONNECTIONS=5, POOL_MAX_CONNECTIONS=100, POOL_ACQUIRE_TIMEOUT=30). Implement builder pattern for pool configuration with sensible defaults. Add support for connection string parsing with production PostgreSQL cluster credentials. Configure connection timeout handling (30s default) and implement proper connection string validation.

## 2. Implement Connection Retry Logic with Exponential Backoff [pending]
### Dependencies: 6.1
### Description: Add robust connection retry mechanism to handle temporary database unavailability during startup or network issues
### Details:
Create a new retry module in crates/database/src/retry.rs implementing exponential backoff strategy. Add configurable retry parameters (max_retries: 5, initial_delay: 1s, max_delay: 30s, multiplier: 2.0). Integrate retry logic into DatabasePool::new() method. Add proper logging for each retry attempt with delay information. Handle different error types appropriately (connection vs authentication errors).

## 3. Validate pgvector Extension and Schema Migration [pending]
### Dependencies: 6.1
### Description: Enhance migration system to validate pgvector extension with 3072-dimension support and verify database schema integrity
### Details:
Update crates/database/src/migrations.rs to check pgvector version and validate 3072-dimension vector support. Add schema validation queries to verify documents and document_sources tables with all constraints. Implement migration state tracking to detect partial migrations. Add rollback capability for failed migrations. Create comprehensive schema verification including indexes, constraints, and enum types.

## 4. Implement Database Health Check Endpoints [pending]
### Dependencies: 6.1, 6.2
### Description: Create comprehensive health check system for Kubernetes readiness and liveness probes
### Details:
Add /health/ready endpoint in crates/mcp/src/server.rs checking database connectivity and migration status. Implement /health/live endpoint for basic service liveness. Add detailed health check in DatabasePool including connection pool metrics (active/idle connections). Implement health check caching (5s TTL) to reduce database load. Return appropriate HTTP status codes (200 for healthy, 503 for unhealthy) with JSON response including details.

## 5. Add Connection Pool Monitoring and Metrics [pending]
### Dependencies: 6.1, 6.4
### Description: Implement connection pool observability with metrics and logging for production monitoring
### Details:
Add connection pool metrics tracking (active_connections, idle_connections, wait_time, acquisition_errors). Implement periodic pool status logging (every 60s) at INFO level. Add connection lifecycle hooks for detailed tracing. Create pool saturation alerts when connections exceed 80% capacity. Export metrics in Prometheus-compatible format at /metrics endpoint. Add connection leak detection with automatic cleanup.


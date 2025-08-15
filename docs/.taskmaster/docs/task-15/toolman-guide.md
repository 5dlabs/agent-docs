# Toolman Guide: Task 14 - Production Monitoring and Observability

## Overview

This task implements comprehensive production monitoring including Prometheus metrics, structured JSON logging, OpenTelemetry tracing, and performance profiling endpoints. The focus is on complete observability with minimal performance impact.

## Core Tools

### Filesystem Server Tools

#### read_file

**Purpose**: Analyze existing code structure and monitoring patterns
**When to Use**:

- Examine current logging and error handling patterns
- Study existing HTTP server implementation for metrics integration
- Review database query patterns for instrumentation
- Analyze existing middleware for correlation ID integration

#### write_file

**Purpose**: Create monitoring infrastructure and configuration
**When to Use**:

- Implement metrics module with Prometheus integration
- Create OpenTelemetry configuration and tracing setup
- Write profiling endpoint implementations
- Add Grafana dashboard configurations

#### edit_file

**Purpose**: Integrate observability into existing codebase
**When to Use**:

- Add metrics instrumentation to existing handlers
- Modify database queries to include tracing spans
- Update HTTP server to include correlation ID middleware
- Integrate structured logging throughout application

### Kubernetes Tools

#### kubernetes_getResource

**Purpose**: Examine existing deployment configuration for monitoring integration
**When to Use**:

- Review current service configuration for metrics endpoints
- Check existing resource limits and monitoring setup
- Validate ServiceMonitor configuration
- Examine current logging and tracing infrastructure

#### kubernetes_listResources

**Purpose**: Discover monitoring infrastructure components
**When to Use**:

- Find existing Prometheus and Grafana deployments
- Locate monitoring namespace resources
- Identify current logging pipeline components
- Check for existing Jaeger tracing infrastructure

## Implementation Flow

### Phase 1: Metrics Infrastructure

1. Add Prometheus dependencies and create metrics module
2. Define custom metrics for key operations
3. Integrate metrics registry into server state
4. Add /metrics endpoint for Prometheus scraping

### Phase 2: Structured Logging

1. Configure tracing-subscriber with JSON formatter
2. Implement correlation ID middleware
3. Replace existing logging with structured format
4. Ensure correlation propagation across operations

### Phase 3: Distributed Tracing

1. Add OpenTelemetry dependencies and configuration
2. Instrument key operations with tracing spans
3. Configure Jaeger exporter and sampling
4. Add trace context propagation headers

### Phase 4: Performance Monitoring

1. Instrument database queries with timing metrics
2. Add per-tool performance tracking
3. Monitor embedding generation performance
4. Track connection pool and resource metrics

### Phase 5: Profiling and Debugging

1. Implement pprof-compatible profiling endpoints
2. Add authentication and rate limiting
3. Create debugging documentation
4. Test profiling in production-like environment

## Best Practices

### Metrics Collection

- Use histograms for timing data with appropriate buckets
- Apply consistent labeling for dashboard filtering
- Keep metrics cardinality reasonable to avoid memory issues
- Use counters for events and gauges for current state

### Structured Logging

- Include correlation IDs in all log entries
- Use consistent field names across all components
- Avoid logging sensitive information
- Structure logs for easy parsing and searching

### Distributed Tracing

- Keep spans focused on meaningful operations
- Use appropriate sampling to control overhead
- Include relevant context in span attributes
- Maintain parent-child relationships correctly

### Performance Impact

- Monitor observability overhead regularly
- Use efficient serialization for structured logs
- Implement async metrics collection where possible
- Test performance impact under load

## Task-Specific Implementation Guidelines

### 1. Metrics Module Structure

```rust
// crates/mcp/src/metrics.rs
use prometheus::{
    register_histogram_with_registry, register_counter_with_registry,
    register_gauge_with_registry, Histogram, Counter, Gauge, Registry
};

pub struct ApplicationMetrics {
    pub query_latency: Histogram,
    pub embedding_generation_time: Histogram,
    pub cache_hit_rate: Counter,
    pub active_connections: Gauge,
    pub request_counter: Counter,
}

impl ApplicationMetrics {
    pub fn new(registry: &Registry) -> Self {
        // Initialize metrics with registry
    }
}
```

### 2. Correlation ID Middleware

```rust
use tower_http::trace::TraceLayer;
use uuid::Uuid;

pub fn correlation_id_layer() -> TraceLayer<SharedClassifier> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request<Body>| {
            let correlation_id = request
                .headers()
                .get("X-Correlation-ID")
                .and_then(|h| h.to_str().ok())
                .unwrap_or_else(|| &Uuid::new_v4().to_string());

            tracing::info_span!(
                "http_request",
                correlation_id = %correlation_id,
                method = %request.method(),
                path = %request.uri().path()
            )
        })
}
```

### 3. Database Query Instrumentation

```rust
// In database queries
#[tracing::instrument(skip(self))]
pub async fn vector_search(
    &self,
    query: &str,
    limit: i64,
) -> Result<Vec<Document>, DatabaseError> {
    let start = std::time::Instant::now();

    let result = sqlx::query_as!()
        .fetch_all(&self.pool)
        .await;

    // Record metrics
    self.metrics.query_latency
        .observe(start.elapsed().as_secs_f64());

    result
}
```

### 4. Profiling Endpoints

```rust
// Profiling endpoint implementation
pub async fn cpu_profile(
    Extension(auth): Extension<AuthState>,
    Query(params): Query<ProfileParams>,
) -> Result<Response<Body>, StatusCode> {
    if !auth.can_profile() {
        return Err(StatusCode::FORBIDDEN);
    }

    let duration = Duration::from_secs(params.seconds.unwrap_or(30));
    let profile = pprof::ProfilerGuard::new(100).unwrap();

    tokio::time::sleep(duration).await;

    match profile.report().build() {
        Ok(report) => {
            let mut body = Vec::new();
            report.pprof().unwrap().write_to_vec(&mut body).unwrap();
            Ok(Response::new(Body::from(body)))
        },
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

## Troubleshooting

### Common Issues

#### Metrics Collection Problems

- Verify Prometheus endpoint accessibility
- Check metric registration and naming
- Monitor memory usage from high cardinality metrics
- Validate histogram bucket configuration

#### Tracing Performance Impact

- Adjust sampling rate for high-traffic endpoints
- Monitor trace export performance
- Check Jaeger backend capacity
- Validate span lifecycle management

#### Correlation ID Propagation

- Ensure middleware order in HTTP stack
- Verify async context propagation
- Check database client span creation
- Validate header extraction and injection

## Validation Steps

### Development Testing

1. **Metrics Endpoint**: Verify Prometheus format and values
2. **Correlation Tracking**: Test ID propagation across operations
3. **Tracing Integration**: Validate spans in Jaeger UI
4. **Profiling**: Test CPU and memory profiling endpoints

### Production Validation

1. **Performance Impact**: Measure overhead under load
2. **Alerting**: Test SLA violation detection
3. **Dashboard**: Verify Grafana visualization
4. **Log Aggregation**: Confirm structured log processing

## Success Indicators

- Prometheus metrics endpoint provides comprehensive system visibility
- Structured logs enable efficient troubleshooting
- Distributed tracing shows complete request flows
- Performance profiling assists with optimization
- Observability overhead remains under 5%
- SLA monitoring and alerting operational
- Operations team can debug issues effectively
- Production performance insights drive optimization

# Autonomous Agent Prompt: Production Monitoring and Observability

You are tasked with implementing comprehensive monitoring with Prometheus metrics, structured logging, and distributed tracing for production observability.

## Your Mission

Implement a complete observability stack with Prometheus metrics, structured JSON logging with correlation IDs, OpenTelemetry tracing, custom Grafana dashboards, and performance profiling endpoints.

## Execution Steps

### Step 1: Set up Prometheus Metrics Infrastructure
- Add prometheus and prometheus-hyper crates to Cargo.toml
- Create `crates/mcp/src/metrics.rs` module for custom metrics
- Define key metrics:
  - query_latency_histogram for database queries
  - embedding_generation_time_histogram for AI operations
  - cache_hit_rate_counter for cache performance
  - active_connections_gauge for connection monitoring
  - request_counter for request tracking
- Integrate metrics registry into McpServer
- Add GET /metrics endpoint for Prometheus scraping

### Step 2: Implement Structured JSON Logging
- Configure tracing-subscriber with json formatter
- Add correlation ID middleware to generate X-Correlation-ID headers
- Implement correlation ID propagation through async operations
- Add structured fields: service_name, version, environment, timestamp
- Replace existing logging with structured JSON format
- Ensure correlation IDs propagate through database queries

### Step 3: Add OpenTelemetry Tracing with Jaeger
- Add opentelemetry, opentelemetry-jaeger, tracing-opentelemetry crates
- Configure OpenTelemetry pipeline with Jaeger exporter
- Create spans for key operations:
  - Database queries in queries.rs
  - Embedding generation operations
  - Tool execution in handlers.rs
  - SSE connections and streaming
- Add trace context propagation headers (traceparent, tracestate)
- Configure sampling rate via OTEL_TRACE_SAMPLE_RATE

### Step 4: Create Custom Performance Metrics
- Instrument DocumentQueries methods with timing metrics
- Add per-tool metrics in McpHandler::handle_tool_call
- Track embedding generation performance with batch timing
- Monitor database connection pool metrics (active, idle, pending)
- Create custom histogram buckets (0.1s, 0.5s, 1s, 2s, 5s)
- Export metrics with labels for Grafana dashboard filtering

### Step 5: Implement Performance Profiling Endpoints
- Add pprof crate for CPU and memory profiling
- Create /debug/pprof/profile endpoint for CPU profiling
- Add /debug/pprof/heap endpoint for memory snapshots
- Implement authentication middleware for profiling endpoints
- Add rate limiting to prevent profiling abuse
- Configure profiling via ENABLE_PROFILING environment variable

## Required Outputs

1. **Prometheus Metrics System** with custom metrics and /metrics endpoint
2. **Structured Logging Infrastructure** with JSON format and correlation IDs
3. **OpenTelemetry Integration** with Jaeger for distributed tracing
4. **Performance Monitoring** with detailed timing and resource metrics
5. **Profiling Endpoints** for production debugging and optimization

## Key Technical Requirements

1. **Metrics Coverage**: All critical operations instrumented
2. **Correlation Tracking**: Request tracing across all components
3. **Performance Impact**: < 5% overhead from observability
4. **Security**: Profiling endpoints protected and rate limited
5. **Integration**: Compatible with existing Kubernetes deployment

## Tools at Your Disposal

- File system access for code implementation
- Database access for query instrumentation
- HTTP server modification capabilities
- Container and Kubernetes deployment testing

## Success Criteria

Your implementation is complete when:
- Prometheus metrics endpoint exposes comprehensive system metrics
- Structured logging provides searchable JSON logs with correlation IDs
- OpenTelemetry tracing shows end-to-end request flows
- Performance metrics enable SLA monitoring and alerting
- Profiling endpoints assist with production debugging
- All observability features integrate seamlessly with existing system

## Important Implementation Notes

- Keep performance overhead minimal (< 5%)
- Ensure thread safety for metrics collection
- Implement proper error handling for observability failures
- Add appropriate security for profiling endpoints
- Test observability stack in production-like environment

## Validation Commands

```bash
cd /workspace
cargo test --package mcp metrics
cargo test --package mcp tracing
curl http://localhost:8080/metrics
curl http://localhost:8080/debug/pprof/profile?seconds=10
```

Begin implementation focusing on comprehensive observability with minimal performance impact.## Quality Gates and CI/CD Process

- Run static analysis after every new function is written:
  - Command: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - Fix all warnings before proceeding to write the next function.
- Before submission, ensure the workspace is clean:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - `cargo test --all-features`
- Feature branch workflow and CI gating:
  - Do all work on a feature branch (e.g., `feature/<task-id>-<short-name>`).
  - Push to the remote feature branch and monitor the GitHub Actions workflow (`.github/workflows/build-server.yml`) until it is green.
  - Require the deployment stage to complete successfully before creating a pull request.
  - Only create the PR after the workflow is green and deployment has succeeded; otherwise fix issues and re-run.

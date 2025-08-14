# Autonomous Agent Prompt: Load Testing and Performance Optimization

You are tasked with conducting comprehensive load testing to validate performance targets and optimize system bottlenecks using k6 or vegeta for 100+ concurrent connections.

## Your Mission

Create a comprehensive load testing suite, identify performance bottlenecks, implement optimization strategies including query result caching, vector similarity optimization, and request coalescing.

## Execution Steps

### Step 1: Create Load Testing Suite
- Examine existing `scripts/load_test_sse.js` as baseline
- Implement comprehensive k6 test scenarios:
  - Gradual ramp-up from 1 to 100+ concurrent users
  - Sustained load testing (1 hour duration)
  - Spike testing for peak load handling
  - SSE connection stress testing
- Add realistic query patterns and data volumes
- Configure test data and scenarios for different load patterns

### Step 2: Database Query Optimization
- Identify slow queries using EXPLAIN ANALYZE
- Optimize vector similarity search with approximation techniques:
  - Implement HNSW (Hierarchical Navigable Small World) indexes
  - Add IVF (Inverted File) indexes for large datasets
  - Configure approximate nearest neighbor search
- Add query result caching with configurable TTL
- Optimize database connection pooling parameters

### Step 3: Application Performance Optimization
- Implement request coalescing for duplicate queries:
  - Cache identical queries in-flight
  - Deduplicate embedding generation requests
  - Share results across multiple concurrent requests
- Add connection pooling optimization for HTTP clients
- Implement lazy loading for embedding models
- Add streaming optimizations for large result sets

### Step 4: Memory and CPU Profiling Under Load
- Profile application during load testing using pprof
- Identify memory leaks and excessive allocations
- Analyze CPU hotspots and optimization opportunities
- Implement memory pool optimizations for frequent allocations
- Add garbage collection tuning for consistent performance

### Step 5: Performance Monitoring and SLA Validation
- Measure and document p50/p95/p99 latencies
- Test connection limit handling (1000+ concurrent connections)
- Verify memory stability during extended load testing
- Validate cache effectiveness and hit rates
- Implement automated performance regression testing

## Required Outputs

1. **Load Testing Suite** with k6 scenarios for various load patterns
2. **Database Optimizations** including vector search improvements
3. **Caching Infrastructure** with TTL and invalidation strategies
4. **Request Coalescing System** for duplicate query handling
5. **Performance Benchmarks** with SLA validation results

## Key Technical Requirements

1. **Throughput**: Handle 100+ concurrent connections
2. **Latency**: p95 < 2 seconds, p99 < 5 seconds
3. **Stability**: No memory leaks during 1-hour sustained load
4. **Efficiency**: Cache hit rate > 70% for repeated queries
5. **Scalability**: Linear performance scaling with resources

## Load Testing Scenarios

```javascript
// k6 load testing configuration
export let options = {
  stages: [
    { duration: '2m', target: 10 },   // Ramp up
    { duration: '5m', target: 50 },   // Stay at 50 users
    { duration: '2m', target: 100 },  // Ramp up to 100
    { duration: '10m', target: 100 }, // Sustained load
    { duration: '2m', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<2000', 'p(99)<5000'],
    http_req_failed: ['rate<0.1'],
  },
};
```

## Tools at Your Disposal

- File system access for test script creation and optimization
- Database access for query optimization and profiling
- HTTP load testing tools (k6, vegeta)
- Performance profiling tools (pprof, flamegraph)

## Success Criteria

Your optimization is complete when:
- Load testing suite validates system performance under stress
- Database queries optimized for sub-second response times
- Caching system reduces database load by 70%+
- Request coalescing eliminates duplicate processing
- System handles 100+ concurrent connections stably
- Memory usage remains stable during extended testing

## Performance Targets

- **Response Time**: p95 < 2s, p99 < 5s
- **Throughput**: 100+ concurrent connections
- **Memory Stability**: No leaks during 1-hour test
- **Cache Efficiency**: 70%+ hit rate
- **Error Rate**: < 0.1% failed requests

## Validation Commands

```bash
cd /workspace
k6 run scripts/load_test_comprehensive.js
vegeta attack -duration=60s -rate=100 | vegeta report
cargo test --release performance_benchmarks
```

Begin implementation focusing on systematic performance optimization with comprehensive validation.## Quality Gates and CI/CD Process

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

# Toolman Guide: Task 15 - Load Testing and Performance Optimization

## Overview

This task implements comprehensive load testing and performance optimization to validate system performance under stress and optimize bottlenecks. Focus areas include database optimization, caching, request coalescing, and memory stability.

## Core Tools

### Filesystem Server Tools

#### read_file
**Purpose**: Analyze existing performance patterns and load testing infrastructure
**When to Use**: 
- Examine existing `scripts/load_test_sse.js` for baseline patterns
- Study current database query implementations for optimization opportunities
- Review existing caching mechanisms and performance monitoring
- Analyze memory allocation patterns in hot code paths

#### write_file
**Purpose**: Create load testing scripts and performance optimization implementations
**When to Use**:
- Implement k6 load testing scenarios with comprehensive coverage
- Create database optimization scripts and index configurations
- Write caching layer implementation with TTL and invalidation
- Add request coalescing logic and memory optimization code

#### edit_file
**Purpose**: Optimize existing code for performance and add instrumentation
**When to Use**:
- Add performance instrumentation to existing query handlers
- Optimize database connection pooling and query patterns
- Integrate caching into existing query flows
- Add memory profiling and leak detection instrumentation

### Kubernetes Tools

#### kubernetes_getResource
**Purpose**: Examine deployment configuration for performance optimization
**When to Use**:
- Review current resource limits and scaling configuration
- Check existing performance monitoring setup
- Validate load balancer configuration for testing
- Examine current deployment scaling policies

#### kubernetes_listResources
**Purpose**: Discover performance testing infrastructure components
**When to Use**:
- Find existing monitoring and metrics collection systems
- Locate load testing infrastructure and tools
- Identify performance testing namespaces and resources
- Check for existing horizontal pod autoscaling configuration

## Implementation Flow

### Phase 1: Load Testing Infrastructure
1. Create comprehensive k6 test scenarios for various load patterns
2. Implement performance monitoring and metrics collection
3. Set up automated test execution pipeline
4. Configure performance thresholds and alerting

### Phase 2: Database Performance Optimization
1. Identify slow queries using database profiling tools
2. Implement vector similarity search optimizations
3. Add query result caching with intelligent invalidation
4. Optimize database connection pooling parameters

### Phase 3: Application Performance Tuning
1. Implement request coalescing for duplicate queries
2. Add memory allocation optimization for high-frequency operations
3. Optimize HTTP client connection pooling
4. Implement lazy loading and streaming optimizations

### Phase 4: Memory and Resource Management
1. Add comprehensive memory profiling during load testing
2. Identify and resolve memory leaks
3. Optimize garbage collection for consistent performance
4. Implement resource cleanup and lifecycle management

### Phase 5: Performance Validation and Monitoring
1. Execute comprehensive load testing scenarios
2. Validate SLA compliance under various load conditions
3. Implement performance regression testing
4. Create operational runbooks for performance troubleshooting

## Best Practices

### Load Testing Strategy
- Start with realistic user patterns and data volumes
- Gradually increase load to identify breaking points
- Test different scenarios (ramp-up, sustained, spike)
- Monitor both application and infrastructure metrics

### Database Optimization
- Use EXPLAIN ANALYZE to identify query bottlenecks
- Implement appropriate indexing strategies
- Consider read replicas for read-heavy workloads
- Monitor connection pool utilization and tune accordingly

### Caching Implementation
- Cache at multiple levels (query results, embeddings, computed values)
- Implement intelligent cache invalidation strategies
- Monitor cache hit rates and adjust TTL accordingly
- Use cache warming for predictable query patterns

### Memory Management
- Profile memory usage under realistic load conditions
- Implement object pooling for frequently allocated objects
- Monitor garbage collection performance and tune parameters
- Use streaming for large data processing where possible

## Task-Specific Implementation Guidelines

### 1. k6 Load Testing Configuration
```javascript
// scripts/load_test_comprehensive.js
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

export let options = {
  stages: [
    { duration: '2m', target: 10 },
    { duration: '5m', target: 50 },
    { duration: '2m', target: 100 },
    { duration: '10m', target: 100 },
    { duration: '2m', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(95)<2000', 'p(99)<5000'],
    http_req_failed: ['rate<0.01'],
    sse_connection_time: ['p(95)<1000'],
  },
};
```

### 2. Database Query Optimization
```sql
-- HNSW index for vector similarity search
CREATE INDEX embeddings_hnsw_idx ON embeddings 
USING hnsw (embedding vector_cosine_ops) 
WITH (m = 16, ef_construction = 64);

-- IVF index for large datasets
CREATE INDEX embeddings_ivf_idx ON embeddings 
USING ivfflat (embedding vector_cosine_ops) 
WITH (lists = 100);
```

### 3. Request Coalescing Implementation
```rust
use std::collections::HashMap;
use tokio::sync::{Mutex, oneshot};

pub struct QueryCoalescer {
    in_flight: Mutex<HashMap<String, Vec<oneshot::Sender<QueryResult>>>>,
}

impl QueryCoalescer {
    pub async fn execute_or_wait(&self, query: String) -> Result<QueryResult> {
        let mut in_flight = self.in_flight.lock().await;
        
        if let Some(waiters) = in_flight.get_mut(&query) {
            let (tx, rx) = oneshot::channel();
            waiters.push(tx);
            drop(in_flight);
            rx.await.map_err(|_| "Query cancelled")
        } else {
            in_flight.insert(query.clone(), Vec::new());
            drop(in_flight);
            
            let result = self.execute_query(&query).await;
            
            let mut waiters = self.in_flight.lock().await.remove(&query).unwrap_or_default();
            for waiter in waiters {
                let _ = waiter.send(result.clone());
            }
            
            result
        }
    }
}
```

### 4. Caching Layer Implementation
```rust
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct QueryCache {
    cache: RwLock<HashMap<String, (QueryResult, Instant)>>,
    ttl: Duration,
}

impl QueryCache {
    pub async fn get_or_execute<F, Fut>(&self, key: &str, executor: F) -> Result<QueryResult>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<QueryResult>>,
    {
        // Try cache first
        if let Some(result) = self.get_cached(key).await {
            return Ok(result);
        }
        
        // Execute and cache result
        let result = executor().await?;
        self.set_cached(key, result.clone()).await;
        Ok(result)
    }
}
```

### 5. Memory Profiling Integration
```rust
#[cfg(feature = "profiling")]
pub fn start_memory_profiling() {
    let guard = pprof::ProfilerGuard::new(100).unwrap();
    
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        
        match guard.report().build() {
            Ok(report) => {
                let file = std::fs::File::create("heap.pb").unwrap();
                let profile = report.pprof().unwrap();
                profile.write_to_writer(file).unwrap();
            }
            Err(e) => eprintln!("Profiling error: {}", e),
        }
    });
}
```

## Troubleshooting

### Load Testing Issues

#### High Response Times
- Check database query performance and indexes
- Verify connection pool sizing and configuration
- Monitor resource utilization (CPU, memory, network)
- Analyze application bottlenecks with profiling

#### Memory Growth During Testing
- Enable memory profiling to identify leak sources
- Check for unclosed database connections
- Verify proper cleanup of temporary objects
- Monitor garbage collection frequency and effectiveness

#### Connection Exhaustion
- Tune database connection pool sizing
- Implement connection retry logic with backoff
- Monitor connection lifecycle and cleanup
- Check for connection leaks in error scenarios

### Performance Optimization Issues

#### Cache Ineffectiveness
- Analyze query patterns and cache key design
- Adjust TTL based on data change frequency
- Monitor cache hit/miss ratios
- Implement cache warming strategies

#### Database Performance Degradation
- Validate index usage with EXPLAIN ANALYZE
- Monitor database resource utilization
- Check for lock contention and long-running queries
- Consider query optimization and rewriting

## Validation Steps

### Load Testing Validation
1. **Baseline Testing**: Establish performance baselines
2. **Ramp-up Testing**: Validate gradual load handling
3. **Sustained Load**: Test extended operation under load
4. **Spike Testing**: Verify handling of traffic bursts

### Performance Optimization Validation
1. **Query Performance**: Measure database query improvements
2. **Cache Effectiveness**: Validate cache hit rates and performance
3. **Memory Stability**: Confirm no leaks during extended testing
4. **Resource Utilization**: Monitor efficient resource usage

### Quality Assurance
```bash
# Execute comprehensive load testing
k6 run --out influxdb scripts/load_test_comprehensive.js

# Performance benchmarking
cargo bench performance_benchmarks

# Memory leak detection
valgrind --tool=memcheck --leak-check=full ./target/release/doc-server

# Cache performance testing
cargo test cache_performance --release
```

## Success Indicators

- Load testing validates 100+ concurrent connection handling
- Response times consistently meet p95 < 2s, p99 < 5s targets
- Database query performance improved by 50%+ through optimization
- Cache hit rates > 70% for repeated query patterns
- Memory usage stable during 1-hour sustained load testing
- Request coalescing eliminates duplicate processing effectively
- Performance regression testing prevents degradation
- System handles spike loads gracefully with quick recovery
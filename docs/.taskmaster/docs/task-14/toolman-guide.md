# Toolman Guide: Task 13 - Container Image Optimization

## Overview

This task optimizes Docker images for production deployment through multi-stage builds, security hardening, binary optimization, and comprehensive scanning integration. The focus is on achieving minimal image size, zero security vulnerabilities, and operational excellence.

## Core Tools

### Filesystem Server Tools

Essential for container optimization, Dockerfile enhancement, and security scanning integration.

#### read_file

**Purpose**: Analyze current container configuration and examine existing patterns
**When to Use**:

- Examine current Dockerfile structure and optimization opportunities
- Study existing signal handling patterns in the codebase
- Review CI/CD pipeline configurations for integration points
- Analyze current security scanning approaches

**Usage Example**:

```
read_file("/workspace/Dockerfile")
read_file("/workspace/crates/mcp/src/http_server.rs")
read_file("/workspace/.github/workflows/deploy-doc-server.yml")
```

#### write_file

**Purpose**: Create optimized Dockerfile and security scanning scripts
**When to Use**:

- Implement enhanced Dockerfile with cargo-chef and distroless base
- Create security scanning scripts with Trivy integration
- Write signal handling implementation for graceful shutdown
- Add CI/CD enhancements for automated security scanning

**Usage Example**:

```
write_file("/workspace/Dockerfile.optimized", "FROM lukemathwalker/cargo-chef...")
write_file("/workspace/scripts/scan_image.sh", "#!/bin/bash\ntrivy image...")
```

#### edit_file

**Purpose**: Modify existing files to integrate optimization features
**When to Use**:

- Add signal handling to existing http_server.rs
- Update Cargo.toml with size optimization settings
- Modify CI/CD workflows to include security scanning
- Update configuration files for production deployment

**Usage Example**:

```
edit_file("/workspace/crates/mcp/src/http_server.rs", add_graceful_shutdown)
edit_file("/workspace/Cargo.toml", add_optimization_settings)
```

### Kubernetes Tools

Essential for validating container optimization in Kubernetes environments.

#### kubernetes_listResources

**Purpose**: List deployed resources to understand current container usage
**When to Use**:

- Finding existing deployments that will use optimized images
- Discovering related services and configurations
- Understanding resource organization and namespaces
- Checking for current security policies

**Parameters**:

- `type`: Resource type (deployments, pods, services)
- `namespace`: Target namespace for doc-server deployment

#### kubernetes_getResource

**Purpose**: Retrieve specific deployment configurations for optimization validation
**When to Use**:

- Examining current deployment specifications
- Reviewing resource limits and security contexts
- Checking container image references
- Validating health check configurations

**Parameters**:

- `type`: Resource type (deployment, service, configmap)
- `name`: Resource name (doc-server)
- `namespace`: Target namespace

#### kubernetes_describeResource

**Purpose**: Get detailed information about container runtime and performance
**When to Use**:

- Checking container startup times and resource usage
- Troubleshooting deployment issues with optimized images
- Validating security contexts and user configurations
- Monitoring graceful shutdown behavior

**Parameters**:

- `kind`: Resource kind (Pod, Deployment)
- `name`: Resource name
- `namespace`: Target namespace

### Documentation Query Tool

#### rust_query

**Purpose**: Query existing Rust documentation for optimization patterns
**When to Use**:

- Understanding existing signal handling implementations
- Finding performance optimization examples
- Learning about container-specific Rust patterns
- Researching graceful shutdown best practices

**Parameters**:

- `query`: Search terms for Rust optimization patterns
- `limit`: Number of results to analyze

## Implementation Flow

### Phase 1: Current State Analysis

1. Use `read_file` to examine existing Dockerfile and build process
2. Use `kubernetes_getResource` to understand current deployment configuration
3. Use `rust_query` to research optimization patterns and best practices
4. Analyze current image size and security posture

### Phase 2: Dockerfile Optimization

1. Implement cargo-chef for dependency caching optimization
2. Migrate to distroless base image for security hardening
3. Add binary optimization settings and UPX compression
4. Test each optimization stage for functionality and performance

### Phase 3: Application Enhancement

1. Implement graceful shutdown signal handling
2. Add health check endpoints compatible with distroless
3. Optimize startup performance and resource usage
4. Test application behavior with optimizations

### Phase 4: Security and Scanning Integration

1. Create security scanning scripts with Trivy
2. Integrate scanning into CI/CD pipeline
3. Configure SBOM generation and compliance reporting
4. Test security scanning with various vulnerability scenarios

### Phase 5: Kubernetes Validation

1. Deploy optimized containers to Kubernetes cluster
2. Validate performance, security, and operational characteristics
3. Test graceful shutdown in orchestrated environment
4. Monitor and measure optimization improvements

## Best Practices

### Container Optimization

- Use multi-stage builds to minimize final image size
- Leverage cargo-chef for optimal dependency layer caching
- Apply binary optimization judiciously to balance size and performance
- Test functionality thoroughly after each optimization step

### Security Hardening

- Use distroless base images to minimize attack surface
- Run containers as non-root users
- Scan for vulnerabilities at every build step
- Generate and maintain SBOM for compliance

### Operational Excellence

- Implement proper signal handling for graceful shutdown
- Add comprehensive health checks for orchestration
- Log important lifecycle events for debugging
- Test deployment scenarios thoroughly

### Performance Monitoring

- Measure startup time impact of optimizations
- Monitor memory usage and resource efficiency
- Benchmark build time improvements
- Validate application performance under load

## Task-Specific Implementation Guidelines

### 1. cargo-chef Multi-Stage Build

```dockerfile
FROM lukemathwalker/cargo-chef:latest-rust-1.70 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release
RUN strip target/release/doc-server
RUN upx --best target/release/doc-server
```

### 2. Distroless Runtime Configuration

```dockerfile
FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/target/release/doc-server /usr/local/bin/
USER nonroot:nonroot
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/local/bin/doc-server", "--health-check"]
ENTRYPOINT ["/usr/local/bin/doc-server"]
```

### 3. Graceful Shutdown Implementation

```rust
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```

### 4. Security Scanning Script

```bash
#!/bin/bash
# scripts/scan_image.sh
IMAGE_NAME=${1:-doc-server:latest}

# Vulnerability scanning with exit on CRITICAL/HIGH
trivy image --exit-code 1 --severity HIGH,CRITICAL $IMAGE_NAME

# Generate SARIF for tooling integration
trivy image --format sarif --output results.sarif $IMAGE_NAME

# Generate SBOM for compliance
trivy image --format spdx-json --output sbom.spdx.json $IMAGE_NAME

echo "Security scan completed for $IMAGE_NAME"
```

### 5. Size Optimization Settings

```toml
# Cargo.toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable link-time optimization
codegen-units = 1   # Better optimization
panic = "abort"     # Reduce binary size
```

## Troubleshooting

### Common Container Issues

#### cargo-chef Build Failures

- Verify cargo-chef version compatibility with Rust version
- Check recipe.json generation includes all workspace dependencies
- Ensure dependency stage has access to all required files
- Monitor Docker layer caching effectiveness

#### Distroless Runtime Problems

- Verify all required shared libraries are available
- Check that application doesn't depend on shell utilities
- Ensure health check works without external dependencies
- Validate user permissions for nonroot user

#### Binary Optimization Issues

- Test functionality after each optimization step
- Monitor startup performance impact
- Verify UPX compression doesn't break functionality
- Check memory usage patterns with optimized binary

#### Signal Handling Problems

- Test signal handling in containerized environment
- Verify signals reach application process (PID 1)
- Check graceful shutdown timeout configuration
- Monitor database connection cleanup

### Security Scanning Issues

#### Trivy Scanning Failures

- Verify Trivy installation and database updates
- Check network connectivity for vulnerability database
- Monitor scan performance for large images
- Handle false positives with appropriate filters

#### SBOM Generation Problems

- Verify SPDX-JSON format compatibility
- Check completeness of dependency information
- Monitor output file size and content
- Validate compliance with organizational requirements

## Validation Steps

### Development Testing

1. **Build Performance**: Measure cargo-chef cache effectiveness
2. **Image Size**: Verify final image size < 100MB
3. **Functionality**: Test all API endpoints in optimized container
4. **Security**: Run vulnerability scans and verify clean results

### Integration Testing

1. **Kubernetes Deployment**: Deploy optimized image to cluster
2. **Health Checks**: Verify readiness and liveness probes
3. **Graceful Shutdown**: Test termination handling
4. **Performance**: Measure startup time and resource usage

### Quality Assurance

```bash
# Build optimized container
docker build -t doc-server:optimized .

# Verify size target
docker images doc-server:optimized

# Run security scan
./scripts/scan_image.sh doc-server:optimized

# Test functionality
docker run --rm -d -p 8080:8080 --name test-container doc-server:optimized
curl http://localhost:8080/health
docker stop test-container  # Test graceful shutdown
```

## Success Indicators

- Container image size reduced to < 100MB
- Zero CRITICAL and HIGH security vulnerabilities
- Build time improvement > 80% with cargo-chef
- Startup time consistently < 5 seconds
- Graceful shutdown completes within 30 seconds
- Security scanning integrated into CI/CD pipeline
- SBOM generation working for compliance
- Kubernetes deployment successful with optimized images
- Application functionality preserved through all optimizations

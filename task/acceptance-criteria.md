# Acceptance Criteria: Task 14 - Container Image Optimization

## Functional Requirements

### 1. cargo-chef Dependency Caching Implementation
- [ ] cargo-chef installed and configured in builder stage
- [ ] Chef stage created with `cargo chef prepare` generating recipe.json
- [ ] Dependencies stage implemented with `cargo chef cook` for cached builds
- [ ] Builder stage modified to use pre-built dependencies
- [ ] Dockerfile layers optimized for maximum cache utilization
- [ ] Build time reduction verified: 80%+ improvement for code-only changes
- [ ] Recipe.json properly captures all workspace dependencies

### 2. Distroless Runtime Migration
- [ ] Runtime base image changed to gcr.io/distroless/cc-debian12
- [ ] All apt-get installations removed from runtime stage
- [ ] Required shared libraries (libssl, libpq) available or statically linked
- [ ] HEALTHCHECK updated to use application's /health endpoint (no curl dependency)
- [ ] USER directive set to nonroot:nonroot compatible with distroless
- [ ] Container startup and functionality validated with distroless base
- [ ] Attack surface reduced by removing shell and package managers

### 3. Binary Optimization and Compression
- [ ] Cargo.toml configured with size optimization settings:
  - [ ] opt-level = "z" for maximum size optimization
  - [ ] lto = true for link-time optimization
  - [ ] codegen-units = 1 for better optimization
  - [ ] panic = "abort" to reduce binary size
- [ ] Strip command applied in builder stage to remove debug symbols
- [ ] UPX compression applied with --best flag
- [ ] Binary size reduction achieved: 60-70% from original
- [ ] Compressed binary functionality verified (startup, API responses, shutdown)
- [ ] Performance impact minimal (< 5% startup time increase)

### 4. Graceful Shutdown and Signal Handling
- [ ] Signal handlers implemented in `crates/mcp/src/http_server.rs`:
  - [ ] SIGTERM handler for orchestration compatibility
  - [ ] SIGINT handler for development convenience
- [ ] Graceful shutdown logic implemented:
  - [ ] Database connections closed cleanly
  - [ ] In-flight HTTP requests completed (with timeout)
  - [ ] Server shutdown sequence orderly and logged
- [ ] Shutdown timeout set to 30 seconds maximum
- [ ] STOPSIGNAL SIGTERM added to Dockerfile
- [ ] Signal handling works correctly with non-root user
- [ ] Kubernetes termination handling verified

### 5. Security Scanning Pipeline Integration
- [ ] `scripts/scan_image.sh` created with Trivy integration:
  - [ ] Vulnerability scanning with severity thresholds
  - [ ] CRITICAL and HIGH vulnerabilities must be zero
  - [ ] SARIF format output for tooling integration
  - [ ] SPDX-JSON SBOM generation for compliance
- [ ] GitHub Action workflow enhanced with security scanning:
  - [ ] Automated scanning on image builds
  - [ ] Build failure on security threshold violations
  - [ ] Artifact storage for scan results and SBOMs
- [ ] Local development scanning support
- [ ] Security remediation documentation provided

## Non-Functional Requirements

### 1. Performance Requirements
- [ ] Final container image size < 100MB
- [ ] Container startup time < 5 seconds
- [ ] First request response time < 2 seconds after startup
- [ ] Memory usage minimal (< 50MB at idle)
- [ ] Build time with cache hits < 2 minutes
- [ ] Cold build time (no cache) < 10 minutes

### 2. Security Requirements
- [ ] Zero CRITICAL vulnerabilities in final image
- [ ] Zero HIGH vulnerabilities in final image
- [ ] MEDIUM and LOW vulnerabilities documented and accepted
- [ ] SBOM generated for all image builds
- [ ] Base image updates automated and tested
- [ ] No sensitive information in image layers
- [ ] Container runs as non-root user

### 3. Operational Requirements
- [ ] Graceful shutdown completes within 30 seconds
- [ ] Health check endpoint responds correctly
- [ ] Container restarts cleanly without data loss
- [ ] Logging maintained during shutdown sequence
- [ ] Resource cleanup verified (connections, file handles)
- [ ] Kubernetes deployment compatibility maintained

## Test Cases

### Test Case 1: cargo-chef Build Optimization
**Given**: Clean Docker build environment
**When**: Build performed twice (first cold, second with code change only)
**Then**:
- First build completes successfully
- Second build reuses dependency layer from cache
- Build time reduction > 80% for code-only changes
- Final binary identical in both builds

### Test Case 2: Distroless Security and Functionality
**Given**: Container built with distroless base image
**When**: Container started and tested
**Then**:
- Container starts successfully as nonroot user
- Health check endpoint returns 200 OK
- API endpoints respond correctly
- No shell or package manager accessible
- Required shared libraries present and functional

### Test Case 3: Binary Optimization Results
**Given**: Binary optimization settings applied
**When**: Binary size measured and functionality tested
**Then**:
- Binary size reduced by 60-70% from original
- UPX compression applied successfully
- Application startup time < 5 seconds
- All API functionality works correctly
- Memory usage remains optimal

### Test Case 4: Graceful Shutdown Handling
**Given**: Running container with active connections
**When**: SIGTERM signal sent to container
**Then**:
- Shutdown signal received and logged
- In-flight requests completed or timed out
- Database connections closed cleanly
- Container exits with code 0
- Shutdown completes within 30 seconds

### Test Case 5: Security Scanning Integration
**Given**: Container image built with potential vulnerabilities
**When**: Security scanning executed
**Then**:
- Trivy scan completes successfully
- CRITICAL and HIGH vulnerabilities fail the build
- SARIF and SBOM outputs generated
- Scan results stored for review
- Build process stops on security failures

### Test Case 6: Kubernetes Deployment Compatibility
**Given**: Optimized container deployed in Kubernetes
**When**: Pod lifecycle events occur (start, stop, restart)
**Then**:
- Pod starts successfully with readiness checks
- Graceful shutdown works with terminationGracePeriodSeconds
- Health checks pass consistently
- Resource limits respected
- No memory leaks or resource exhaustion

## Deliverables Checklist

### Container Optimization
- [ ] Enhanced Dockerfile with multi-stage cargo-chef build
- [ ] Distroless runtime configuration
- [ ] Binary optimization and compression settings
- [ ] Size and security optimized final image

### Application Changes
- [ ] Graceful shutdown implementation in http_server.rs
- [ ] Signal handling for SIGTERM and SIGINT
- [ ] Health check endpoint for container orchestration
- [ ] Logging enhancements for operational visibility

### Security and Operations
- [ ] Security scanning scripts and CI/CD integration
- [ ] SBOM generation for compliance
- [ ] Vulnerability remediation documentation
- [ ] Operational runbook for container management

### Testing and Validation
- [ ] Container functionality tests
- [ ] Performance benchmark results
- [ ] Security scan reports and approval
- [ ] Kubernetes deployment validation

## Validation Criteria

### Automated Testing
```bash
# Build and size validation
docker build -t doc-server:optimized .
docker images doc-server:optimized | grep -E '<\s*100MB'

# Security scanning
./scripts/scan_image.sh doc-server:optimized
echo $?  # Must return 0 (no CRITICAL/HIGH vulnerabilities)

# Functionality testing
docker run -d --name test-container doc-server:optimized
docker exec test-container /usr/local/bin/doc-server --health-check
docker stop test-container  # Test graceful shutdown
```

### Manual Validation
1. **Performance Testing**: Measure startup time and memory usage
2. **Security Review**: Review scan results and SBOM contents
3. **Operational Testing**: Test in Kubernetes environment
4. **Build Process**: Validate cache efficiency and build times
5. **Signal Handling**: Test graceful shutdown under various conditions

## Definition of Done

Task 13 is complete when:

1. **Image Optimization**: Container size < 100MB with full functionality
2. **Security Hardening**: Zero CRITICAL/HIGH vulnerabilities with SBOM generation
3. **Performance Targets**: Startup time < 5 seconds, graceful shutdown < 30 seconds
4. **Build Optimization**: cargo-chef provides 80%+ build time improvement
5. **Signal Handling**: Graceful shutdown works correctly in all environments
6. **Integration Testing**: All functionality validated in containerized environment
7. **Documentation Complete**: Security and operational procedures documented

## Success Metrics

- Container image size reduction > 60% from baseline
- Build time improvement > 80% for code-only changes
- Zero security vulnerabilities at CRITICAL and HIGH levels
- Startup performance < 5 seconds consistently
- Graceful shutdown success rate 100% within timeout
- Resource usage minimal (< 50MB idle memory)
- Security scan integration blocks vulnerable deployments
- Operational documentation enables team self-service### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact

# Autonomous Agent Prompt: Container Image Optimization

You are tasked with optimizing Docker images for size, security, and performance with multi-stage builds, minimal attack surface, and comprehensive security scanning integration.

## Your Mission

Transform the existing Dockerfile to achieve production-ready container images with cargo-chef optimization, distroless runtime, binary compression, graceful shutdown handling, and integrated security scanning.

## Execution Steps

### Step 1: Implement cargo-chef for Dependency Caching
- Examine current Dockerfile structure and dependency handling
- Install cargo-chef in builder stage for optimal dependency caching
- Create dedicated chef stage with `cargo chef prepare` for recipe.json generation
- Add dependencies stage with `cargo chef cook` for separate dependency building
- Modify builder stage to use pre-built dependencies from chef stage
- Verify build time improvements for code-only changes

### Step 2: Migrate to Distroless Runtime Image
- Replace current debian:bookworm-slim with gcr.io/distroless/cc-debian12
- Remove apt-get installations from runtime stage (unnecessary in distroless)
- Ensure required shared libraries (libssl, libpq) are available or statically linked
- Update HEALTHCHECK to use application's built-in /health endpoint
- Configure USER directive compatible with distroless nonroot user
- Test container startup and functionality with distroless base

### Step 3: Add Binary Optimization and Compression
- Configure Cargo.toml with size optimization settings:
  - opt-level = "z" for size optimization
  - lto = true for link-time optimization
  - codegen-units = 1 for better optimization
  - panic = "abort" to reduce binary size
- Add strip command in builder stage to remove debug symbols
- Install and apply UPX compression with --best flag
- Target 60-70% binary size reduction while maintaining performance
- Verify compressed binary functionality

### Step 4: Implement Graceful Shutdown and Signal Handling
- Navigate to `crates/mcp/src/http_server.rs`
- Implement tokio::signal handlers for SIGTERM and SIGINT
- Add graceful shutdown logic:
  - Close database connections cleanly
  - Complete in-flight HTTP requests
  - Shutdown embedding service connections
- Set 30-second timeout for shutdown sequence
- Update Dockerfile with STOPSIGNAL SIGTERM
- Ensure signal handling works with non-root user

### Step 5: Integrate Security Scanning Pipeline
- Create `scripts/scan_image.sh` with Trivy vulnerability scanning
- Configure severity thresholds (CRITICAL and HIGH must be zero)
- Add GitHub Action workflow for automated security scanning
- Generate SBOM (Software Bill of Materials) for compliance
- Add scanning to CI/CD pipeline with failure conditions
- Document security scanning process and remediation procedures

## Required Outputs

Generate these optimization artifacts:

1. **Enhanced Dockerfile** with cargo-chef, distroless, and optimizations
2. **Signal Handling Code** in http_server.rs for graceful shutdown
3. **Security Scanning Scripts** and CI/CD integration
4. **Build Configuration** optimized for size and security
5. **Documentation** covering security and operational procedures

## Key Technical Requirements

1. **Size Target**: Final image size < 100MB
2. **Security**: Zero CRITICAL and HIGH vulnerabilities
3. **Performance**: Startup time < 5 seconds
4. **Reliability**: Graceful shutdown within 30 seconds
5. **Compliance**: SBOM generation and vulnerability tracking

## Dockerfile Structure Requirements

```dockerfile
# Multi-stage build with cargo-chef
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

FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/target/release/doc-server /usr/local/bin/
USER nonroot:nonroot
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/local/bin/doc-server", "--health-check"]
ENTRYPOINT ["/usr/local/bin/doc-server"]
```

## Security Scanning Configuration

```bash
#!/bin/bash
# scripts/scan_image.sh
trivy image --exit-code 1 --severity HIGH,CRITICAL doc-server:latest
trivy image --format sarif --output results.sarif doc-server:latest
trivy image --format spdx-json --output sbom.spdx.json doc-server:latest
```

## Tools at Your Disposal

- File system access for Dockerfile and script modifications
- Container build and testing capabilities
- Security scanning tools integration
- Performance measurement and optimization tools

## Success Criteria

Your optimization is complete when:
- cargo-chef reduces rebuild times for code-only changes by 80%+
- Distroless base image reduces attack surface significantly
- Binary optimization achieves target size reduction
- Graceful shutdown handles all signals properly within timeout
- Security scanning prevents deployment of vulnerable images
- All performance and size targets are consistently met

## Important Implementation Notes

- Test container functionality thoroughly after each optimization
- Verify all required shared libraries are available in distroless
- Ensure graceful shutdown works correctly in Kubernetes environment
- Validate security scanning integration doesn't break CI/CD pipeline
- Monitor startup performance to ensure optimizations don't degrade it

## Signal Handling Implementation

```rust
// In crates/mcp/src/http_server.rs
use tokio::signal;

pub async fn run_server_with_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server().await?;
    
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    
    if let Err(e) = graceful.await {
        eprintln!("Server error: {}", e);
    }
    
    Ok(())
}

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

    println!("Shutdown signal received, starting graceful shutdown...");
}
```

## Validation Commands

Before completion, run:
```bash
cd /workspace
docker build -t doc-server:optimized .
docker images doc-server:optimized  # Verify size < 100MB
./scripts/scan_image.sh doc-server:optimized
docker run --rm doc-server:optimized --health-check
```

Begin optimization focusing on security, performance, and operational excellence.## Quality Gates and CI/CD Process

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

## Worktree and Parallel Branching (Required for parallel tasks)

- Use a dedicated Git worktree and feature branch for this task to avoid conflicts with other parallel tasks.

### Steps
1. Create a worktree and feature branch for this task:
```bash
git worktree add ../agent-docs-task-14 -b feature/task-14-<short-name>
```
2. Enter the worktree and do all work from there:
```bash
cd ../agent-docs-task-14
```
3. Develop in this isolated directory, follow Quality Gates (run clippy pedantic after each new function; fmt/clippy/tests before pushing).
4. Push and monitor GitHub Actions; only create the PR after CI is green and deployment succeeds.
5. When finished:
```bash
git worktree list
git worktree remove ../agent-docs-task-14
```

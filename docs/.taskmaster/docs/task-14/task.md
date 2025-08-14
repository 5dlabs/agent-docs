# Task 14: Container Registry and CI/CD Pipeline

## Overview
Implement comprehensive CI/CD pipeline with container registry integration for automated building, testing, and deployment of the Doc Server.

## Implementation Guide
- Configure GitHub Actions workflow for automated builds
- Implement container image building with multi-stage Dockerfile
- Set up GitHub Container Registry (GHCR) integration
- Add automated testing pipeline with quality gates
- Configure deployment automation to Kubernetes

## Technical Requirements
- GitHub Actions CI/CD workflow
- Multi-stage Dockerfile optimization
- Container registry integration (GHCR)
- Automated testing and quality gates
- Kubernetes deployment automation

## Success Metrics
- Automated builds triggered on code changes
- Container images built and published successfully
- Quality gates prevent broken deployments
- Deployment automation works reliably
- Build and deployment times within acceptable limits## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.

## Detailed Requirements to Meet Acceptance Criteria

1) Docker Optimization (already present)
- Maintain `Dockerfile.optimized` with cargo-chef prepare/cook layers and distroless runtime; ensure UPX and strip for smallest binary.
- Record image size and build time in CI logs for baseline tracking.

2) Graceful Shutdown & Health
- Keep `--health-check` flag; HEALTHCHECK in distroless stage references it; ensure readiness/liveness endpoints are wired in chart.

3) Security Scanning
- Use `scripts/scan_image.sh` in CI after image build; fail pipeline on HIGH/CRITICAL vulnerabilities; upload SARIF + SBOM as artifacts.

4) CI Workflow (GitHub Actions)
- Jobs:
  - `lint-test`: `cargo fmt --check`, `cargo clippy -D warnings -W clippy::pedantic`, `cargo test --all-features`.
  - `build-image`: build with `Dockerfile.optimized`, tag `ghcr.io/<org>/agent-docs:<sha>`; push on main/feature branches.
  - `scan-image`: run Trivy via `scripts/scan_image.sh` on pushed tag; upload reports; gate on zero HIGH/CRITICAL.
  - `deploy`: authenticate to cluster; `helm upgrade --install` using Task 13 chart and environment-specific values.

5) Measurements
- CI should print final image size, cold and warm build durations, and confirm cargo-chef cache hits (to demonstrate 80% improvement for code-only changes).


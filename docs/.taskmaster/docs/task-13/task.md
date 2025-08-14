# Task 13: Kubernetes Deployment Configuration

## Overview
Create and optimize Helm charts for production Kubernetes deployment with proper resource allocation, security policies, and high availability configuration.

## Implementation Guide
- Create comprehensive Helm chart structure
- Define configurable deployment parameters
- Implement proper resource limits and requests
- Add security policies and network configurations
- Configure high availability and auto-scaling

## Technical Requirements
- Helm chart with values.yaml configuration (ensure existence; recreate if missing)
- Kubernetes manifests (Deployment, Service, Ingress)
- Resource limits (CPU: 500m-2000m, Memory: 512Mi-2Gi)
- ConfigMap and Secret management
- PodDisruptionBudget and auto-scaling policies

## Notes from Assessment
- Ensure Helm `values.yaml` is restored (file was missing) and includes envs:
  - `VECTOR_DATABASE_URL`, `DATABASE_URL`, `OPENAI_API_KEY`, `DOC_SERVER_CONFIG_PATH`
- CI/CD must deploy via Helm as part of 4-step validation

## Success Metrics
- Successful deployment to production cluster
- Resource utilization within defined limits
- High availability with zero-downtime deployments
- Proper security policy enforcement
- Auto-scaling responds to load changes## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.

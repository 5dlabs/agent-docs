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

## Detailed Requirements to Meet Acceptance Criteria

1) Chart Structure
- Required files:
  - `Chart.yaml`, `values.yaml`
  - `templates/deployment.yaml`, `templates/service.yaml`, `templates/ingress.yaml`
  - `templates/configmap.yaml`, `templates/secret.yaml` (or ExternalSecret if applicable)
  - `templates/pdb.yaml`, `templates/hpa.yaml`, `templates/serviceaccount.yaml`, `templates/networkpolicy.yaml` (optional but recommended)

2) values.yaml Contents
- Image repo/tag, pullPolicy; resources (requests/limits) within guidelines.
- Environment:
  - `DATABASE_URL`, `VECTOR_DATABASE_URL` (if used), `OPENAI_API_KEY` (Secret), `DOC_SERVER_CONFIG_PATH`, `PORT`, `RUST_LOG`.
- Probes: http GET `/_/health` or equivalent; readiness/liveness with appropriate thresholds.
- SecurityContext: runAsNonRoot, readOnlyRootFilesystem, fsGroup if needed (for non-distroless variants).
- HPA policy: target CPU/Memory utilization, min/max replicas.
- PDB minAvailable or maxUnavailable to ensure HA.

3) Deployment Manifests
- Use non-root user, readOnlyRootFilesystem when possible; mount config as `ConfigMap` and secrets via `Secret` or External Secrets.
- Set probes to match server health endpoint and port.
- Enable topology spread or anti-affinity for HA.

4) Network and Security
- `NetworkPolicy` limiting ingress to expected namespaces/selectors.
- Optional `PodSecurity`/`PSa` alignment per cluster policy.

5) Documentation
- `README` with installation, required values, and example overrides.
- Example `values.override.yaml` for production and staging.

6) CI/CD Hooks
- Workflow steps to package/chart-lint (e.g., `helm lint`), render templates (`helm template`), and deploy with selected context/environment.


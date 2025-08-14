# Autonomous Agent Prompt: Kubernetes Deployment Configuration

## Mission
Create and optimize Helm charts for production Kubernetes deployment with proper resource allocation and security policies.

## Success Criteria
- [ ] Helm chart includes `values.yaml` and `templates/` for: Deployment, Service, Ingress, ConfigMap/Secret (or ExternalSecret), PDB, HPA, ServiceAccount, NetworkPolicy
- [ ] Resource requests/limits defined (CPU 500m–2000m; Memory 512Mi–2Gi)
- [ ] SecurityContext configured (runAsNonRoot, readOnlyRootFilesystem) as applicable
- [ ] High availability (probes, PDB, HPA) configured and validated

## Deployment Validation (Mandatory 4-step)
1. Push branch to GitHub to trigger CI
2. CI builds container image and runs clippy/tests
3. Deploy via Helm using chart values (ensure `values.yaml` exists and probes map correctly)
4. Perform real-world validation with a compliant MCP client

## Implementation Steps
1. Create `values.yaml` capturing image, resources, env, probes, securityContext, autoscaling, PDB
2. Author `templates/` listed in Success Criteria; ensure readiness/liveness probes hit the server health endpoint/port
3. Add annotations/labels for tracing and release tracking
4. Provide `README` and sample overrides for staging/production
5. Add CI steps to lint (`helm lint`) and template (`helm template`) the chart

## Quality Gates and CI/CD Process

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


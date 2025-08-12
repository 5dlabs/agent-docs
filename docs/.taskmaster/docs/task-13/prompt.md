# Autonomous Agent Prompt: Kubernetes Deployment Configuration

## Mission
Create and optimize Helm charts for production Kubernetes deployment with proper resource allocation and security policies.

## Success Criteria
- [ ] Helm charts created with configurable parameters
- [ ] Resource limits properly configured
- [ ] Security policies implemented
- [ ] High availability configuration complete

## Deployment Validation (Mandatory 4-step)
1. Push branch to GitHub to trigger CI
2. CI builds container image and runs clippy/tests
3. Deploy via Helm using chart values (ensure `values.yaml` exists)
4. Perform real-world validation with a compliant MCP client
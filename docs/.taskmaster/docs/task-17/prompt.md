# Autonomous Agent Prompt: CI/CD Pipeline Enhancement

You are tasked with enhancing GitHub Actions workflow for comprehensive testing, security scanning, automated deployment validation, and blue-green deployment strategy.

## Your Mission

Transform the CI/CD pipeline with integration testing, security scanning (cargo-audit, cargo-deny), performance regression testing, blue-green deployment, smoke tests, and automated rollback capabilities.

## Execution Steps

### Step 1: Add Integration Test Stage with Database Fixtures

- Examine current `.github/workflows/deploy-doc-server.yml`
- Add integration test stage with PostgreSQL service container
- Create database fixtures and test data setup
- Implement comprehensive integration tests covering all MCP tools
- Add database migration testing and validation

### Step 2: Implement Security Scanning Integration

- Add cargo-audit for vulnerability scanning of dependencies
- Integrate cargo-deny for license compliance and security policies
- Add Trivy container image scanning to pipeline
- Configure SAST (Static Application Security Testing) tools
- Add dependency vulnerability reporting and alerting

### Step 3: Add Performance Regression Testing

- Integrate performance benchmarks into CI pipeline
- Add baseline performance measurement and comparison
- Implement automated performance regression detection
- Configure performance alerts for significant degradation
- Add load testing integration for major releases

### Step 4: Implement Blue-Green Deployment Strategy

- Design blue-green deployment workflow for Kubernetes
- Add deployment validation and health checking
- Implement traffic switching logic with zero downtime
- Add rollback automation on deployment failure
- Configure deployment status monitoring and alerting

### Step 5: Add Post-Deployment Validation

- Implement comprehensive smoke tests post-deployment
- Add endpoint health verification and functional testing
- Configure monitoring integration and alert validation
- Add automated rollback on smoke test failures
- Implement deployment success/failure notifications

## Required Outputs

1. **Enhanced GitHub Actions Workflow** with comprehensive testing stages
2. **Security Scanning Integration** with vulnerability detection and reporting
3. **Performance Testing Pipeline** with regression detection
4. **Blue-Green Deployment Configuration** with automated rollback
5. **Smoke Testing Suite** with post-deployment validation

## Key Technical Requirements

1. **Pipeline Speed**: Total pipeline execution < 10 minutes
2. **Security**: Zero tolerance for CRITICAL vulnerabilities
3. **Reliability**: Automated rollback on any deployment failure
4. **Visibility**: Comprehensive status reporting and notifications
5. **Compliance**: License scanning and policy enforcement

## Tools at Your Disposal

- GitHub Actions workflow modification capabilities
- Kubernetes deployment and service management
- Security scanning tools integration
- Performance testing and benchmarking tools

## Success Criteria

Your enhancement is complete when:

- Integration tests validate all functionality with real database
- Security scanning prevents vulnerable deployments
- Performance regression testing catches degradation automatically
- Blue-green deployment enables zero-downtime updates
- Smoke tests validate deployment success comprehensively
- Pipeline execution time remains under 10 minutes

Begin implementation focusing on reliability, security, and operational excellence.## Quality Gates and CI/CD Process

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

- Use Git worktrees to isolate this task's working directory and feature branch to avoid conflicts with other tasks running in parallel.

### Steps

1. Create a dedicated worktree and feature branch for this task:

2. Enter the worktree and do all work from there:

3. Run your development session here (e.g., Claude Code) and follow the Quality Gates section (Clippy pedantic after each new function; fmt/clippy/tests before pushing).

4. Push from this worktree and monitor GitHub Actions; create a PR only after CI is green and deployment succeeds.

5. Manage worktrees when finished:
   /Users/jonathonfritz/code/work-projects/5dlabs/agent-docs 610a801 [main]

# Acceptance Criteria: Basic Security for Single-User Environment

## Functional Requirements

### 1. Lightweight Protections

- [ ] Origin validation and localhost-only binding
- [ ] Request size limits enforced (e.g., 1–2 MB)
- [ ] Basic token-bucket rate limiting
- [ ] Secrets via environment variables; no secrets in logs
- [ ] Minimal audit logs for tool invocations and security events

### 2. Header and Input Hygiene

- [ ] Security headers reasonable for local use (nosniff, frame options)
- [ ] Input validation for key parameters; parameterized queries only
- [ ] Redaction of sensitive values in logs

## Non-Functional Requirements

- [ ] No P0/P1 vulnerabilities in basic automated scan
- [ ] Overhead from protections keeps p95 latency within Task 15 targets
- [ ] Configuration documented in `requirements.yaml`

## Test Cases

- [ ] Invalid Origin blocked with 403
- [ ] Oversized request rejected with 413
- [ ] Rate limit breach yields 429 and logs event
- [ ] Logs contain no API keys or secrets

## Definition of Done

- Protections enabled by default
- Documentation updated for local/single-user scope
- Validated via 4-step deployment/test loop### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact

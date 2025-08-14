# Autonomous Agent Prompt: Security Hardening and Compliance

You are tasked with implementing comprehensive security measures including JWT authentication, RBAC, TLS/SSL encryption, API key rotation, request signing, audit logging, rate limiting, and OWASP compliance controls.

## Your Mission

Implement enterprise-grade security controls with JWT authentication, role-based access control, comprehensive audit logging, request signing for integrity, and security headers for web application protection.

## Execution Steps

### Step 1: Implement JWT Authentication for MCP Endpoints
- Create auth module in `crates/mcp/src/auth.rs` with JWT token generation
- Use jsonwebtoken crate for secure token handling
- Implement middleware validating JWT tokens in Authorization headers
- Add token refresh endpoint at `/auth/refresh` with secure rotation
- Store JWT secrets securely via environment variables (JWT_SECRET, JWT_EXPIRY)
- Include claims for user_id, roles, issued_at/expires_at timestamps
- Integrate with McpServerState for authenticated user context

### Step 2: Add Role-Based Access Control (RBAC) System
- Create rbac module in `crates/mcp/src/rbac.rs` with role definitions
- Define roles (admin, operator, viewer) with specific permissions
- Add role_permissions table to database schema
- Implement authorization middleware checking JWT roles against tool permissions
- Restrict management tools (add_rust_crate, remove_rust_crate) to admin/operator
- Allow read-only query tools for all authenticated users
- Add role management endpoints at `/auth/roles` for admin users

### Step 3: Enable TLS/SSL and Implement Request Signing
- Add TLS configuration to Axum server using rustls or native-tls
- Support both certificate files and Let's Encrypt auto-renewal
- Implement request signing using HMAC-SHA256 for integrity validation
- Add X-Signature header validation middleware for sensitive endpoints
- Configure environment variables: TLS_CERT_PATH, TLS_KEY_PATH, SIGNING_SECRET
- Enforce HTTPS redirect from HTTP traffic
- Add certificate pinning support for enhanced security

### Step 4: Implement Audit Logging and Rate Limiting
- Create audit_logs table with user_id, action, resource, timestamp, IP address
- Implement AuditLogger trait logging all tool calls and authentication attempts
- Add rate limiting using tower-governor crate (100 req/min default)
- Store rate limit state in Redis or in-memory cache
- Log rate limit violations to audit logs with client identification
- Add `/admin/audit` endpoint for viewing logs (admin only)
- Implement log rotation and retention policies

### Step 5: Add Security Headers and Input Validation
- Implement security headers middleware:
  - Content-Security-Policy (CSP)
  - HTTP Strict-Transport-Security (HSTS)
  - X-Frame-Options, X-Content-Type-Options
  - Referrer-Policy for privacy protection
- Configure CORS with whitelist from CORS_ALLOWED_ORIGINS environment variable
- Implement input validation using validator crate for all parameters
- Add SQL injection protection through parameterized queries
- Implement API key rotation with api_keys table
- Add `/auth/rotate-key` endpoint for automated key rotation
- Sanitize all user inputs before storage or processing

## Required Outputs

1. **JWT Authentication System** with secure token handling and rotation
2. **RBAC Implementation** with role-based tool access control
3. **TLS/SSL Configuration** with request signing for integrity
4. **Audit Logging System** with comprehensive activity tracking
5. **Security Headers and Validation** with OWASP compliance

## Key Technical Requirements

1. **Authentication**: JWT-based with secure token rotation
2. **Authorization**: Role-based access control for all tools
3. **Encryption**: TLS/SSL for all communications
4. **Integrity**: Request signing for sensitive operations
5. **Compliance**: OWASP security controls and audit logging

## Security Architecture

```
Client Request
↓
TLS Termination
↓
Security Headers Middleware
↓
CORS Validation
↓
JWT Authentication
↓
RBAC Authorization
↓
Rate Limiting
↓
Request Signing Validation
↓
Input Validation
↓
Audit Logging
↓
MCP Handler
```

## Tools at Your Disposal

- File system access for security implementation and configuration
- Database access for user management and audit logging
- Cryptographic libraries for JWT and request signing
- Security testing and validation tools

## Success Criteria

Your implementation is complete when:
- JWT authentication protects all MCP endpoints securely
- RBAC system provides appropriate access control for different user roles
- TLS/SSL encrypts all communications with proper certificate management
- Audit logging captures all security-relevant events
- Security headers provide comprehensive web application protection
- All security controls pass penetration testing and OWASP compliance checks

## Important Security Notes

- Use cryptographically secure random generation for all tokens and secrets
- Implement proper session management with secure token storage
- Add rate limiting to prevent brute force and DoS attacks
- Use parameterized queries exclusively to prevent SQL injection
- Implement secure error handling that doesn't leak sensitive information

## Security Testing Requirements

- Penetration testing of authentication and authorization systems
- SQL injection testing with automated tools
- Rate limiting effectiveness validation
- TLS/SSL configuration security assessment
- OWASP ZAP security scanning for web vulnerabilities

## Validation Commands

```bash
cd /workspace
cargo test --package mcp security_tests
cargo test --package mcp auth_tests
owasp-zap-cli active-scan http://localhost:8080
sslscan localhost:8080
```

Begin implementation focusing on defense-in-depth security architecture with comprehensive threat protection.## Quality Gates and CI/CD Process

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
/Users/jonathonfritz/code/work-projects/5dlabs/agent-docs  610a801 [main]

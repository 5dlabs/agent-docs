# Toolman Guide: Task 19 - Security Hardening and Compliance

## Overview

This task implements comprehensive security controls including JWT authentication, role-based access control, TLS/SSL encryption, audit logging, rate limiting, and OWASP compliance measures for enterprise-grade security.

## Core Tools

### Filesystem Server Tools

#### read_file
**Purpose**: Analyze existing security implementations and configuration patterns
**When to Use**: 
- Examine current authentication and authorization implementations
- Study existing middleware and security patterns in codebase
- Review current database security and access control mechanisms
- Analyze existing error handling and security logging approaches

#### write_file
**Purpose**: Create comprehensive security infrastructure and configurations
**When to Use**:
- Implement JWT authentication and token management systems
- Create RBAC authorization middleware and role definitions
- Write TLS/SSL configuration and certificate management
- Add audit logging infrastructure and security monitoring

#### edit_file
**Purpose**: Integrate security controls into existing application components
**When to Use**:
- Add authentication middleware to existing HTTP handlers
- Integrate authorization checks into existing tool implementations
- Modify database queries to include audit logging
- Update configuration files with security settings

### Security Server Tools

#### audit_scan
**Purpose**: Perform security audits and vulnerability assessments
**When to Use**:
- Scanning for known vulnerabilities in dependencies
- Validating security configuration compliance
- Checking for common security misconfigurations
- Performing automated security assessments

#### vulnerability_check
**Purpose**: Check for specific security vulnerabilities and weaknesses
**When to Use**:
- Testing authentication bypass vulnerabilities
- Validating authorization controls effectiveness
- Checking for injection vulnerabilities (SQL, XSS)
- Testing rate limiting and DoS protection

#### compliance_report
**Purpose**: Generate compliance reports for security standards
**When to Use**:
- Creating OWASP Top 10 compliance reports
- Generating audit reports for security assessments
- Documenting security control implementation
- Tracking compliance with security policies

### Kubernetes Tools

#### kubernetes_getResource
**Purpose**: Examine existing security configurations in Kubernetes
**When to Use**:
- Review current TLS/SSL certificate configurations
- Check existing RBAC policies and service accounts
- Validate current network policies and security contexts
- Examine existing secret and ConfigMap security

#### kubernetes_listResources
**Purpose**: Discover security-related infrastructure and policies
**When to Use**:
- Finding existing security policies and configurations
- Locating certificate management and PKI infrastructure
- Identifying current monitoring and logging systems
- Checking for existing security scanning and compliance tools

## Implementation Flow

### Phase 1: Authentication Infrastructure
1. Implement JWT token generation and validation system
2. Create authentication middleware for all protected endpoints
3. Add token refresh and rotation mechanisms
4. Integrate authentication with existing application state

### Phase 2: Authorization and RBAC
1. Design role hierarchy and permission system
2. Implement authorization middleware with role checking
3. Create database schema for role and permission management
4. Add role management endpoints for administrative control

### Phase 3: Transport Security
1. Configure TLS/SSL with proper certificate management
2. Implement request signing for message integrity
3. Add security headers for web application protection
4. Configure HTTPS enforcement and security policies

### Phase 4: Monitoring and Logging
1. Implement comprehensive audit logging system
2. Add rate limiting with configurable thresholds
3. Create security event monitoring and alerting
4. Implement log rotation and retention policies

### Phase 5: Input Validation and Compliance
1. Add comprehensive input validation and sanitization
2. Implement OWASP security controls
3. Add API key rotation and management system
4. Create compliance reporting and monitoring

## Best Practices

### Authentication Security
- Use cryptographically secure random number generation
- Implement proper token lifecycle management
- Add token blacklisting for secure logout
- Monitor and alert on authentication anomalies

### Authorization Design
- Follow principle of least privilege
- Implement defense-in-depth authorization layers
- Cache authorization decisions for performance
- Regular review and audit of role assignments

### Cryptographic Implementation
- Use industry-standard algorithms (RS256, HS256)
- Proper key management and rotation procedures
- Secure storage of cryptographic materials
- Regular security assessments of cryptographic implementations

### Logging and Monitoring
- Log all security-relevant events comprehensively
- Implement real-time security monitoring
- Create automated alerting for security incidents
- Regular audit log review and analysis

## Task-Specific Implementation Guidelines

### 1. JWT Authentication System
```rust
// JWT authentication implementation
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub roles: Vec<String>,
    pub exp: usize,
    pub iat: usize,
    pub session_id: String,
}

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtAuth {
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.required_spec_claims.insert("exp".to_string());
        
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
        }
    }
    
    pub fn generate_token(&self, claims: &Claims) -> Result<String> {
        encode(&Header::default(), claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGeneration(e.to_string()))
    }
    
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| AuthError::TokenValidation(e.to_string()))
    }
}
```

### 2. RBAC Authorization System
```rust
// RBAC implementation
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,
    Operator,
    Viewer,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    ReadDocuments,
    WriteDocuments,
    ManageCrates,
    AdministerSystem,
}

pub struct RbacService {
    role_permissions: HashMap<Role, Vec<Permission>>,
}

impl RbacService {
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();
        
        role_permissions.insert(Role::Admin, vec![
            Permission::ReadDocuments,
            Permission::WriteDocuments,
            Permission::ManageCrates,
            Permission::AdministerSystem,
        ]);
        
        role_permissions.insert(Role::Operator, vec![
            Permission::ReadDocuments,
            Permission::WriteDocuments,
            Permission::ManageCrates,
        ]);
        
        role_permissions.insert(Role::Viewer, vec![
            Permission::ReadDocuments,
        ]);
        
        Self { role_permissions }
    }
    
    pub fn check_permission(&self, roles: &[Role], required_permission: Permission) -> bool {
        roles.iter().any(|role| {
            self.role_permissions
                .get(role)
                .map_or(false, |perms| perms.contains(&required_permission))
        })
    }
}
```

### 3. Security Headers Middleware
```rust
// Security headers implementation
use axum::http::{HeaderMap, HeaderName, HeaderValue};

pub fn security_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    
    // Content Security Policy
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
        ),
    );
    
    // HTTP Strict Transport Security
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    
    // X-Frame-Options
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    
    // X-Content-Type-Options
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    
    // Referrer Policy
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    
    headers
}
```

### 4. Audit Logging System
```rust
// Audit logging implementation
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: String,
    pub status_code: Option<u16>,
    pub duration_ms: Option<u64>,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_event(&self, entry: AuditLogEntry) -> Result<(), AuditError>;
    async fn query_logs(
        &self,
        filters: &AuditQueryFilters,
    ) -> Result<Vec<AuditLogEntry>, AuditError>;
}

pub struct DatabaseAuditLogger {
    pool: PgPool,
}

#[async_trait]
impl AuditLogger for DatabaseAuditLogger {
    async fn log_event(&self, entry: AuditLogEntry) -> Result<(), AuditError> {
        sqlx::query!(
            r#"
            INSERT INTO audit_logs (
                id, timestamp, user_id, session_id, action, resource,
                ip_address, user_agent, request_id, status_code, duration_ms, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            entry.id,
            entry.timestamp,
            entry.user_id,
            entry.session_id,
            entry.action,
            entry.resource,
            entry.ip_address.map(|ip| ip.to_string()),
            entry.user_agent,
            entry.request_id,
            entry.status_code.map(|c| c as i32),
            entry.duration_ms.map(|d| d as i64),
            entry.metadata
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::Database(e.to_string()))?;
        
        Ok(())
    }
}
```

### 5. Rate Limiting Implementation
```rust
// Rate limiting with tower-governor
use tower_governor::{
    governor::Governor, key_extractor::KeyExtractor, GovernorConfigBuilder,
};
use std::net::IpAddr;

#[derive(Clone, Debug)]
pub struct IpKeyExtractor;

impl KeyExtractor for IpKeyExtractor {
    type Key = IpAddr;
    
    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, tower_governor::GovernorError> {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|hv| hv.to_str().ok())
            .and_then(|s| s.parse::<IpAddr>().ok())
            .or_else(|| {
                req.extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ci| ci.0.ip())
            })
            .ok_or(tower_governor::GovernorError::UnableToExtractKey)
    }
}

pub fn create_rate_limiter() -> Governor<IpAddr> {
    GovernorConfigBuilder::default()
        .per_second(2) // 2 requests per second
        .burst_size(10) // Allow burst of 10 requests
        .key_extractor(IpKeyExtractor)
        .finish()
        .unwrap()
}
```

## Troubleshooting

### Authentication Issues

#### JWT Token Validation Failures
- Verify JWT secret configuration and environment variables
- Check token expiry and clock synchronization
- Validate JWT algorithm configuration (HS256 vs RS256)
- Monitor token blacklist and revocation status

#### Session Management Problems
- Check session storage and persistence mechanisms
- Validate session timeout and renewal logic
- Monitor concurrent session limits and enforcement
- Review session invalidation on logout

### Authorization Problems

#### Role Assignment Issues
- Verify role-to-permission mapping accuracy
- Check role inheritance and hierarchy logic
- Validate role assignment and modification procedures
- Monitor role-based access control effectiveness

#### Permission Enforcement Failures
- Check authorization middleware integration
- Verify permission checking logic and caching
- Validate tool-specific access control implementation
- Monitor authorization decision audit logging

### Security Configuration Issues

#### TLS/SSL Problems
- Verify certificate validity and expiry
- Check TLS version enforcement and cipher suites
- Validate certificate chain and trust store
- Monitor SSL/TLS handshake performance

#### Rate Limiting Ineffectiveness
- Check rate limit configuration and thresholds
- Verify client identification and key extraction
- Monitor rate limit state storage and persistence
- Validate rate limit bypass and whitelist logic

## Validation Steps

### Security Testing
1. **Authentication Testing**: JWT token validation and bypass attempts
2. **Authorization Testing**: Role-based access control enforcement
3. **Injection Testing**: SQL injection and XSS vulnerability scanning
4. **Transport Security**: TLS/SSL configuration and certificate validation

### Compliance Validation
1. **OWASP Testing**: Top 10 vulnerability assessment
2. **Penetration Testing**: Comprehensive security assessment
3. **Audit Testing**: Logging completeness and accuracy validation
4. **Policy Compliance**: Security policy adherence verification

### Quality Assurance
```bash
# Security testing and validation
cargo test --package mcp security_tests
cargo test --package mcp auth_rbac_tests

# Vulnerability scanning
cargo audit
cargo deny check

# Web application security testing
owasp-zap-cli active-scan http://localhost:8080
nmap -sS -sV localhost

# TLS/SSL testing
sslscan localhost:8080
testssl.sh --fast localhost:8080
```

## Success Indicators

- JWT authentication protects all endpoints with zero bypasses
- RBAC system enforces proper access control with 100% accuracy
- TLS/SSL encryption secures all communications with A+ rating
- Audit logging captures 99%+ of security-relevant events
- Rate limiting prevents abuse with 99%+ effectiveness
- Security headers provide comprehensive web protection
- Input validation prevents injection attacks completely
- OWASP Top 10 compliance verified through testing
- Security incident response time < 15 minutes
- Zero critical vulnerabilities in security assessments
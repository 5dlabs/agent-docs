# Task 19: Basic Security for Single-User Environment

## Overview
Implement practical, lightweight security measures appropriate for a single-user environment (5–6 agents). Emphasize sane defaults without heavy compliance overhead.

## Implementation Guide
- Origin validation and localhost binding for MCP server
- Input size limits and basic request validation
- Rate limiting suitable for single-user (low thresholds)
- Secrets via environment variables; avoid logging sensitive data
- Minimal audit logs for tool invocations and errors

## Technical Requirements
- CORS/Origin checks; DNS rebinding protections
- Request size limits (e.g., 1–2 MB)
- Simple token-bucket rate limiter
- Structured logging with redaction

## Success Metrics
- No P0/P1 vulnerabilities in basic scan
- Blocks cross-origin and oversized request attempts
- No sensitive data in logs
- Stable under expected agent usage
# Task ID: 5
# Title: Protocol Version (Fixed 2025-06-18) and Headers
# Status: pending
# Dependencies: 2
# Priority: medium
# Description: Implement strict MCP protocol header handling for single supported version 2025-06-18 and header compliance. Drop negotiation and legacy support for MVP.
# Details:
Add MCP-Protocol-Version header parsing and validation. Support only protocol version 2025-06-18. Return HTTP 400 for unsupported protocol versions. Implement proper Content-Type headers in responses. Store fixed protocol version in session state for subsequent requests.

# Test Strategy:
Validate header presence and format, verify proper error responses for invalid versions, and ensure fixed version echoed consistently.

# Subtasks:
## 1. Create Protocol Header Utilities [pending]
### Dependencies: None
### Description: Implement constants and simple validators for MCP protocol version (fixed 2025-06-18).
### Details:
Create a new module protocol_version.rs in crates/mcp/src/. Define a ProtocolVersion enum with variants for each supported version (V2025_06_18, V2025_03_26, V2024_11_05). Implement FromStr trait for parsing version strings from headers. Add version comparison methods for determining compatibility. Create a ProtocolRegistry struct to manage supported versions with methods for checking if a version is supported, finding the best matching version, and determining fallback versions. Include constants for current, fallback, and legacy protocol versions. Add comprehensive error types for version-related failures.

## 2. Implement HTTP Header Extraction and Validation [pending]
### Dependencies: 4.1
### Description: Create header extraction middleware for parsing and validating MCP-Protocol-Version and Accept headers from incoming HTTP requests using Axum extractors.
### Details:
Create headers.rs module with custom Axum extractors. Implement McpProtocolVersionHeader extractor using TypedHeader or custom FromRequestParts implementation validating only `2025-06-18`. Create ContentTypeValidator to ensure proper Content-Type headers in responses. Implement header validation logic that returns HTTP 400 Bad Request for invalid or missing required headers. Add header constants and helper functions for header manipulation. Integrate with tracing for logging header processing.

## 3. Set Fixed Version in Initialize Handler [pending]
### Dependencies: 4.1, 4.2
### Description: Ensure initialize handler includes fixed `protocolVersion: "2025-06-18"` and stores it in session state.
### Details:
Modify handle_initialize to set fixed protocol version in response and session state. Validate header if provided and return 400 if not `2025-06-18`.

## 4. Implement Session State Management with Protocol Version [pending]
### Dependencies: 4.3
### Description: Create session state management that stores fixed protocol version and enforces version consistency across subsequent requests.
### Details:
Create session.rs module with SessionState struct containing negotiated_version field and session metadata. Implement SessionStore using Arc<RwLock<HashMap>> for thread-safe session storage keyed by session ID. Add session creation during initialization with generated UUID session ID. Implement session retrieval and validation for subsequent requests. Add session expiry mechanism with configurable TTL. Create middleware to extract session ID from Mcp-Session-Id header and attach session state to request context. Ensure all handlers have access to session state for version-aware processing.

## 5. Add Response Header Management [pending]
### Dependencies: 4.2, 4.4
### Description: Implement response header management to ensure proper Content-Type headers and required MCP headers (fixed protocol version).
### Details:
Create response.rs module with ResponseBuilder that adds appropriate headers based on request context. Implement Content-Type header setting for application/json responses. Add Mcp-Session-Id header to responses during initialization. Implement middleware to automatically add required headers to all responses. Ensure CORS headers are properly maintained alongside MCP-specific headers.


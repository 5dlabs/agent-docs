//! MCP Protocol Version Management
//!
//! This module provides comprehensive protocol version management for MCP (Model Context Protocol).
//! It implements the fixed protocol version "2025-06-18" for MVP with strict validation and
//! centralized version management.

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// The only supported MCP protocol version (fixed for MVP)
pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";

/// MCP Protocol Version enum for type-safe version handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolVersion {
    /// MCP Protocol Version 2025-06-18 (the only supported version in MVP)
    V2025_06_18,
}

impl ProtocolVersion {
    /// Get the string representation of the protocol version
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V2025_06_18 => "2025-06-18",
        }
    }

    /// Check if this is the supported protocol version
    #[must_use]
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::V2025_06_18)
    }

    /// Get the current/supported protocol version
    #[must_use]
    pub const fn current() -> Self {
        Self::V2025_06_18
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ProtocolVersion {
    type Err = ProtocolVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "2025-06-18" => Ok(Self::V2025_06_18),
            other => Err(ProtocolVersionParseError::UnsupportedVersion(
                other.to_string(),
            )),
        }
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::current()
    }
}

/// Protocol version parsing errors
#[derive(Debug, Error)]
pub enum ProtocolVersionParseError {
    #[error("Unsupported protocol version: {0} (only {SUPPORTED_PROTOCOL_VERSION} supported)")]
    UnsupportedVersion(String),
}

/// Protocol version registry for managing supported versions and validation
#[derive(Debug, Clone)]
pub struct ProtocolRegistry {
    /// The current supported version (fixed to 2025-06-18)
    current_version: ProtocolVersion,
}

impl ProtocolRegistry {
    /// Create a new protocol registry with the current supported version
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_version: ProtocolVersion::current(),
        }
    }

    /// Check if a protocol version is supported
    #[must_use]
    pub fn is_version_supported(&self, version: ProtocolVersion) -> bool {
        version.is_supported()
    }

    /// Check if a protocol version string is supported
    #[must_use]
    pub fn is_version_string_supported(&self, version_str: &str) -> bool {
        match ProtocolVersion::from_str(version_str) {
            Ok(version) => self.is_version_supported(version),
            Err(_) => false,
        }
    }

    /// Get the current supported version
    #[must_use]
    pub const fn current_version(&self) -> ProtocolVersion {
        self.current_version
    }

    /// Get the current supported version as string
    #[must_use]
    pub const fn current_version_string(&self) -> &'static str {
        SUPPORTED_PROTOCOL_VERSION
    }

    /// Validate that a version string matches the supported version
    ///
    /// # Errors
    ///
    /// Returns an error if the version string doesn't match the supported version.
    pub fn validate_version_string(
        &self,
        version_str: &str,
    ) -> Result<ProtocolVersion, ProtocolVersionParseError> {
        let version = ProtocolVersion::from_str(version_str)?;
        if self.is_version_supported(version) {
            Ok(version)
        } else {
            Err(ProtocolVersionParseError::UnsupportedVersion(
                version_str.to_string(),
            ))
        }
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_as_str() {
        assert_eq!(ProtocolVersion::V2025_06_18.as_str(), "2025-06-18");
    }

    #[test]
    fn test_protocol_version_display() {
        assert_eq!(ProtocolVersion::V2025_06_18.to_string(), "2025-06-18");
    }

    #[test]
    fn test_protocol_version_is_supported() {
        assert!(ProtocolVersion::V2025_06_18.is_supported());
    }

    #[test]
    fn test_protocol_version_current() {
        assert_eq!(ProtocolVersion::current(), ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_version_default() {
        assert_eq!(ProtocolVersion::default(), ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_version_from_str_valid() {
        let version = ProtocolVersion::from_str("2025-06-18").unwrap();
        assert_eq!(version, ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_version_from_str_invalid() {
        let result = ProtocolVersion::from_str("2024-11-05");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProtocolVersionParseError::UnsupportedVersion(_)
        ));
    }

    #[test]
    fn test_protocol_version_from_str_with_whitespace() {
        // Should trim whitespace
        let result = ProtocolVersion::from_str(" 2025-06-18 ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_registry_new() {
        let registry = ProtocolRegistry::new();
        assert_eq!(registry.current_version(), ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_registry_default() {
        let registry = ProtocolRegistry::default();
        assert_eq!(registry.current_version(), ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_registry_is_version_supported() {
        let registry = ProtocolRegistry::new();
        assert!(registry.is_version_supported(ProtocolVersion::V2025_06_18));
    }

    #[test]
    fn test_protocol_registry_is_version_string_supported() {
        let registry = ProtocolRegistry::new();
        assert!(registry.is_version_string_supported("2025-06-18"));
        assert!(!registry.is_version_string_supported("2024-11-05"));
        assert!(!registry.is_version_string_supported("invalid-version"));
    }

    #[test]
    fn test_protocol_registry_current_version_string() {
        let registry = ProtocolRegistry::new();
        assert_eq!(registry.current_version_string(), "2025-06-18");
    }

    #[test]
    fn test_protocol_registry_validate_version_string_valid() {
        let registry = ProtocolRegistry::new();
        let result = registry.validate_version_string("2025-06-18");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_registry_validate_version_string_invalid() {
        let registry = ProtocolRegistry::new();
        let result = registry.validate_version_string("2024-11-05");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProtocolVersionParseError::UnsupportedVersion(_)
        ));
    }

    #[test]
    fn test_constants_consistency() {
        assert_eq!(SUPPORTED_PROTOCOL_VERSION, "2025-06-18");
        assert_eq!(
            ProtocolVersion::current().as_str(),
            SUPPORTED_PROTOCOL_VERSION
        );
    }
}

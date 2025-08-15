//! Document loading and parsing
//!
//! This crate provides document loading functionality for various documentation
//! types including Rust crates, Jupyter notebooks, and API documentation.

pub mod extractors;
pub mod loaders;
pub mod migration;
pub mod parsers;

pub use loaders::*;
pub use migration::*;

/// Re-export commonly used types
pub use url::Url;

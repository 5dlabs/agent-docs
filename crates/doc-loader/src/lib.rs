//! Document loading and parsing
//! 
//! This crate provides document loading functionality for various documentation
//! types including Rust crates, Jupyter notebooks, and API documentation.

pub mod loaders;
pub mod parsers;
pub mod extractors;

pub use loaders::*;

/// Re-export commonly used types
pub use url::Url;
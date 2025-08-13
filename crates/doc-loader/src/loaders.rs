//! Document loaders for various sources

/// Rust crate documentation loader
#[derive(Default)]
pub struct RustLoader;

impl RustLoader {
    /// Create a new Rust loader
    pub fn new() -> Self {
        Self
    }
}

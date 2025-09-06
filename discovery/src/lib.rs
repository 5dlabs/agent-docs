//! Discovery crate: Claude Code–powered repository analysis producing ingest plans.

mod claude;
mod analyzer;

pub use analyzer::{IngestionStrategy, IntelligentRepositoryAnalyzer, RepositoryAnalysis};

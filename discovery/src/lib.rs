//! Discovery crate: Claude Code–powered repository analysis producing ingest plans.

mod analyzer;
mod claude;

pub use analyzer::{IngestionStrategy, IntelligentRepositoryAnalyzer, RepositoryAnalysis};

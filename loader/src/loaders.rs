//! Document loader types used by the loader CLI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Documentation page emitted by the loader CLI when parsing local files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPage {
    pub url: String,
    pub content: String,
    pub item_type: String, // "markdown", "html", "code", etc.
    pub module_path: String,
    pub extracted_at: DateTime<Utc>,
}


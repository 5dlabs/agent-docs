//! Intelligent metadata extraction for document ingestion
//!
//! This module provides enhanced metadata analysis that examines document content
//! to automatically assign appropriate topics, categories, complexity levels, and formats.
//! This ensures consistent metadata across all ingestion paths (loader CLI, MCP tools, etc.).
//!
//! The metadata extraction is driven by configuration from tools.json, making it flexible
//! and extensible for new document types without code changes.

use serde_json::Value;
use std::collections::HashMap;

/// Metadata hints loaded from tools.json configuration
#[derive(Debug, Clone)]
pub struct MetadataHints {
    pub supported_formats: Vec<String>,
    pub supported_complexity_levels: Vec<String>,
    pub supported_categories: Vec<String>,
    pub supported_topics: Vec<String>,
    pub supports_api_version: bool,
    pub topic_keywords: HashMap<String, Vec<String>>,
    pub category_keywords: HashMap<String, Vec<String>>,
}

impl MetadataHints {
    /// Load metadata hints from tools.json configuration
    ///
    /// # Errors
    ///
    /// Returns an error if tools.json cannot be found or parsed
    pub fn load_from_tools_config() -> Result<HashMap<String, Self>, Box<dyn std::error::Error>> {
        let tools_content = std::fs::read_to_string("tools.json")
            .or_else(|_| std::fs::read_to_string("../tools.json"))
            .or_else(|_| std::fs::read_to_string("../../tools.json"))?;

        let tools_config: Value = serde_json::from_str(&tools_content)?;
        let mut hints_map = HashMap::new();

        if let Some(tools_array) = tools_config["tools"].as_array() {
            for tool in tools_array {
                if let (Some(doc_type), Some(hints)) =
                    (tool["docType"].as_str(), tool["metadataHints"].as_object())
                {
                    let metadata_hints = Self {
                        supported_formats: hints
                            .get("supported_formats")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        supported_complexity_levels: hints
                            .get("supported_complexity_levels")
                            .and_then(|v| v.as_array())
                            .map_or_else(
                                || {
                                    vec![
                                        "beginner".to_string(),
                                        "intermediate".to_string(),
                                        "advanced".to_string(),
                                    ]
                                },
                                |arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                },
                            ),
                        supported_categories: hints
                            .get("supported_categories")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        supported_topics: hints
                            .get("supported_topics")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        supports_api_version: hints
                            .get("supports_api_version")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        topic_keywords: Self::parse_keyword_mappings(hints.get("topic_keywords")),
                        category_keywords: Self::parse_keyword_mappings(
                            hints.get("category_keywords"),
                        ),
                    };
                    hints_map.insert(doc_type.to_string(), metadata_hints);
                }
            }
        }

        Ok(hints_map)
    }

    /// Parse keyword mappings from JSON configuration
    fn parse_keyword_mappings(keywords_value: Option<&Value>) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();

        if let Some(keywords_obj) = keywords_value.and_then(|v| v.as_object()) {
            for (key, value) in keywords_obj {
                if let Some(keywords_array) = value.as_array() {
                    let keywords: Vec<String> = keywords_array
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    mappings.insert(key.clone(), keywords);
                }
            }
        }

        mappings
    }
}

/// Create enhanced metadata by analyzing document content and structure
/// Uses configuration from `tools.json` to determine valid values for each `doc_type`
#[must_use]
pub fn create_enhanced_metadata(
    doc_type: &str,
    source_name: &str,
    content: &str,
    doc_path: &str,
) -> Value {
    let mut metadata = serde_json::Map::new();

    // Basic metadata
    metadata.insert(
        "original_doc_type".to_string(),
        Value::String(doc_type.to_string()),
    );
    metadata.insert("source".to_string(), Value::String(source_name.to_string()));
    metadata.insert(
        "imported_at".to_string(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );

    // Load configuration-driven metadata hints
    if let Ok(hints_map) = MetadataHints::load_from_tools_config() {
        if let Some(hints) = hints_map.get(doc_type) {
            // Use configuration-driven analysis
            create_metadata_from_hints(&mut metadata, content, doc_path, hints);
        } else {
            // Fallback to generic analysis for unconfigured doc_types
            add_generic_metadata(&mut metadata, content, doc_path);
        }
    } else {
        // Fallback if tools.json can't be loaded
        add_generic_metadata(&mut metadata, content, doc_path);
    }

    Value::Object(metadata)
}

/// Create metadata based on configuration hints from tools.json
fn create_metadata_from_hints(
    metadata: &mut serde_json::Map<String, Value>,
    content: &str,
    doc_path: &str,
    hints: &MetadataHints,
) {
    // Determine format
    let format = determine_best_format(doc_path, content, &hints.supported_formats);
    metadata.insert("format".to_string(), Value::String(format));

    // Determine complexity
    if !hints.supported_complexity_levels.is_empty() {
        let complexity =
            determine_best_complexity(content, doc_path, &hints.supported_complexity_levels);
        metadata.insert("complexity".to_string(), Value::String(complexity));
    }

    // Determine topic
    if !hints.supported_topics.is_empty() {
        let topic = determine_best_topic(content, doc_path, &hints.supported_topics, hints);
        metadata.insert("topic".to_string(), Value::String(topic));
    }

    // Determine category
    if !hints.supported_categories.is_empty() {
        let category =
            determine_best_category(content, doc_path, &hints.supported_categories, hints);
        metadata.insert("category".to_string(), Value::String(category));
    }

    // Add API version if supported (would need additional logic to extract from content)
    if hints.supports_api_version {
        if let Some(api_version) = extract_api_version(content) {
            metadata.insert("api_version".to_string(), Value::String(api_version));
        }
    }
}

/// Add generic metadata when no configuration hints are available
fn add_generic_metadata(
    metadata: &mut serde_json::Map<String, Value>,
    content: &str,
    doc_path: &str,
) {
    let format = determine_document_format(doc_path, content);
    metadata.insert("format".to_string(), Value::String(format));

    let complexity = determine_complexity_by_content(content, doc_path);
    metadata.insert("complexity".to_string(), Value::String(complexity));

    // Basic topic analysis
    let topic = if content.to_lowercase().contains("api") {
        "apis"
    } else {
        "general"
    };
    metadata.insert("topic".to_string(), Value::String(topic.to_string()));
}

/// Merge enhanced metadata with existing metadata, preserving existing fields
pub fn merge_enhanced_metadata(
    existing_metadata: &mut Value,
    doc_type: &str,
    source_name: &str,
    content: &str,
    doc_path: &str,
) {
    let enhanced = create_enhanced_metadata(doc_type, source_name, content, doc_path);

    if let (Some(existing_obj), Some(enhanced_obj)) =
        (existing_metadata.as_object_mut(), enhanced.as_object())
    {
        // Only add fields that don't already exist
        for (key, value) in enhanced_obj {
            if !existing_obj.contains_key(key) {
                existing_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Determine best format from supported options
fn determine_best_format(doc_path: &str, content: &str, supported_formats: &[String]) -> String {
    let detected_format = determine_document_format(doc_path, content);

    // If the detected format is in the supported list, use it
    if supported_formats.contains(&detected_format) {
        detected_format
    } else {
        // Otherwise, use the first supported format as default
        supported_formats
            .first()
            .unwrap_or(&"markdown".to_string())
            .clone()
    }
}

/// Determine best topic from supported options based on content analysis
fn determine_best_topic(
    content: &str,
    doc_path: &str,
    supported_topics: &[String],
    hints: &MetadataHints,
) -> String {
    let content_lower = content.to_lowercase();
    let path_lower = doc_path.to_lowercase();

    // Score each supported topic based on keyword matches
    let mut topic_scores: Vec<(String, i32)> = supported_topics
        .iter()
        .map(|topic| {
            let score =
                calculate_topic_score(&content_lower, &path_lower, topic, &hints.topic_keywords);
            (topic.clone(), score)
        })
        .collect();

    // Sort by score (highest first)
    topic_scores.sort_by(|a, b| b.1.cmp(&a.1));

    // Return the highest scoring topic, or first as default
    topic_scores.first().map_or_else(
        || {
            supported_topics
                .first()
                .unwrap_or(&"general".to_string())
                .clone()
        },
        |(topic, _)| topic.clone(),
    )
}

/// Determine best category from supported options based on content analysis  
fn determine_best_category(
    content: &str,
    doc_path: &str,
    supported_categories: &[String],
    hints: &MetadataHints,
) -> String {
    let content_lower = content.to_lowercase();
    let path_lower = doc_path.to_lowercase();

    // Score each supported category based on keyword matches
    let mut category_scores: Vec<(String, i32)> = supported_categories
        .iter()
        .map(|category| {
            let score = calculate_category_score(
                &content_lower,
                &path_lower,
                category,
                &hints.category_keywords,
            );
            (category.clone(), score)
        })
        .collect();

    // Sort by score (highest first)
    category_scores.sort_by(|a, b| b.1.cmp(&a.1));

    // Return the highest scoring category, or first as default
    category_scores.first().map_or_else(
        || {
            supported_categories
                .first()
                .unwrap_or(&"general".to_string())
                .clone()
        },
        |(category, _)| category.clone(),
    )
}

/// Determine best complexity from supported options
fn determine_best_complexity(content: &str, doc_path: &str, supported_levels: &[String]) -> String {
    let detected_complexity = determine_complexity_by_content(content, doc_path);

    // If the detected complexity is supported, use it
    if supported_levels.contains(&detected_complexity) {
        detected_complexity
    } else {
        // Otherwise, use a reasonable default
        if supported_levels.contains(&"intermediate".to_string()) {
            "intermediate".to_string()
        } else {
            supported_levels
                .first()
                .unwrap_or(&"beginner".to_string())
                .clone()
        }
    }
}

/// Calculate topic relevance score based on keyword matches
fn calculate_topic_score(
    content_lower: &str,
    path_lower: &str,
    topic: &str,
    topic_keywords: &HashMap<String, Vec<String>>,
) -> i32 {
    let mut score = 0;

    // Get keywords from configuration, or use topic name as fallback
    let keywords = topic_keywords.get(topic).map_or_else(
        || vec![topic],
        |v| v.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    // Count keyword matches in content and path
    for keyword in keywords {
        if content_lower.contains(keyword) {
            score += 2; // Content matches are worth more
        }
        if path_lower.contains(keyword) {
            score += 1; // Path matches are worth less
        }
    }

    score
}

/// Calculate category relevance score based on keyword matches
fn calculate_category_score(
    content_lower: &str,
    path_lower: &str,
    category: &str,
    category_keywords: &HashMap<String, Vec<String>>,
) -> i32 {
    let mut score = 0;

    // Get keywords from configuration, or use category name as fallback
    let keywords = category_keywords.get(category).map_or_else(
        || vec![category],
        |v| v.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    for keyword in keywords {
        if content_lower.contains(keyword) {
            score += 2;
        }
        if path_lower.contains(keyword) {
            score += 1;
        }
    }

    score
}

/// Extract API version from content (basic implementation)
fn extract_api_version(content: &str) -> Option<String> {
    let content_lower = content.to_lowercase();

    // Look for common API version patterns
    if content_lower.contains("v6") || content_lower.contains("version 6") {
        Some("v6".to_string())
    } else if content_lower.contains("v5") || content_lower.contains("version 5") {
        Some("v5".to_string())
    } else if content_lower.contains("v4") || content_lower.contains("version 4") {
        Some("v4".to_string())
    } else if content_lower.contains("v3") || content_lower.contains("version 3") {
        Some("v3".to_string())
    } else if content_lower.contains("v2") || content_lower.contains("version 2") {
        Some("v2".to_string())
    } else if content_lower.contains("v1") || content_lower.contains("version 1") {
        Some("v1".to_string())
    } else {
        None
    }
}

/// Determine document format from path and content
fn determine_document_format(doc_path: &str, content: &str) -> String {
    use std::path::Path;

    // Check file extension first (case-insensitive)
    let path = Path::new(doc_path);
    if let Some(ext) = path.extension() {
        match ext.to_string_lossy().to_lowercase().as_str() {
            "md" | "markdown" => return "markdown".to_string(),
            "json" => return "json".to_string(),
            "ts" | "tsx" => return "typescript".to_string(),
            "js" | "jsx" => return "javascript".to_string(),
            _ => {}
        }
    }

    // Analyze content structure
    if content.contains("```typescript") || content.contains("```ts") {
        return "typescript".to_string();
    }
    if content.contains("```javascript") || content.contains("```js") {
        return "javascript".to_string();
    }
    if content.contains("```json") || content.trim().starts_with('{') {
        return "json".to_string();
    }
    if content.contains("# ") || content.contains("## ") {
        return "markdown".to_string();
    }

    "markdown".to_string() // Default fallback
}

/// Determine complexity based on content characteristics
fn determine_complexity_by_content(content: &str, doc_path: &str) -> String {
    let content_lower = content.to_lowercase();
    let path_lower = doc_path.to_lowercase();

    if content.len() < 1000
        || content_lower.contains("getting started")
        || content_lower.contains("quick start")
        || content_lower.contains("introduction")
        || content_lower.contains("basic")
        || path_lower.contains("intro")
        || path_lower.contains("basic")
        || path_lower.contains("start")
    {
        "beginner"
    } else if content.len() > 5000
        || content_lower.contains("advanced")
        || content_lower.contains("complex")
        || content.matches("```").count() > 5
        || content_lower.contains("implementation")
        || path_lower.contains("advanced")
        || path_lower.contains("complex")
    {
        "advanced"
    } else {
        "intermediate"
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jupiter_metadata_extraction() {
        let content = r#"# Jupiter API v6 Documentation

This guide covers the Jupiter swap API endpoints for trading tokens on Solana.

## Swap Endpoint

The swap endpoint allows you to exchange tokens:

```typescript
const response = await fetch('/api/swap', {
  method: 'POST',
  body: JSON.stringify({
    inputMint: 'So11111111111111111111111111111111111111112',
    outputMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
    amount: 1000000
  })
});
```

This is an advanced integration guide for developers."#;

        let doc_path = "docs/api/v6/swap.md";
        let metadata = create_enhanced_metadata("jupiter", "test-source", content, doc_path);

        assert_eq!(metadata["topic"], "trading"); // "swap" and "trading" keywords score higher than "api"
        assert_eq!(metadata["category"], "swap-api");
        assert_eq!(metadata["format"], "markdown");
        assert_eq!(metadata["complexity"], "beginner"); // Content length < 1000
    }

    #[test]
    fn test_rust_metadata_extraction() {
        let content = r#"# Tokio Documentation

Tokio is an async runtime for Rust.

```rust
#[tokio::main]
async fn main() {
    println!("Hello, async world!");
}
```"#;

        let doc_path = "tokio/src/lib.rs";
        let metadata = create_enhanced_metadata("rust", "tokio", content, doc_path);

        // "rust" doc_type not in tools.json, so no topic field is set
        assert!(metadata.get("topic").is_none());
        assert_eq!(metadata["format"], "markdown");
        assert_eq!(metadata["complexity"], "beginner");
    }
}

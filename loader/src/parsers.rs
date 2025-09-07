//! Document parsers for multiple formats
//!
//! This module provides comprehensive document parsing capabilities for various
//! documentation formats including Markdown, HTML, JSON API specs, PDF, and more.
//! It includes intelligent content chunking and structure analysis.

use anyhow::{anyhow, Result};
use pulldown_cmark::{html, Options, Parser};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

/// Supported document formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentFormat {
    Markdown,
    Html,
    Json,
    Yaml,
    Toml,
    Pdf,
    PlainText,
    Code,
    ApiSpec,
    Unknown,
}

impl DocumentFormat {
    /// Detect format from file extension
    #[must_use]
    pub fn from_extension(path: &str) -> Self {
        let path_obj = Path::new(path);
        if let Some(ext) = path_obj.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "md" | "markdown" => Self::Markdown,
                "html" | "htm" => Self::Html,
                "json" => Self::Json,
                "yaml" | "yml" => Self::Yaml,
                "toml" => Self::Toml,
                "pdf" => Self::Pdf,
                "txt" => Self::PlainText,
                "rs" | "py" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "h" => Self::Code,
                _ => Self::Unknown,
            }
        } else {
            Self::Unknown
        }
    }

    /// Detect format from content
    #[must_use]
    pub fn from_content(content: &str) -> Self {
        // Check for JSON
        if serde_json::from_str::<Value>(content).is_ok() {
            // Check if it's an API spec
            if content.contains("swagger") || content.contains("openapi") {
                return Self::ApiSpec;
            }
            return Self::Json;
        }

        // Check for YAML
        if serde_yaml::from_str::<Value>(content).is_ok() {
            return Self::Yaml;
        }

        // Check for TOML
        if toml::from_str::<Value>(content).is_ok() {
            return Self::Toml;
        }

        // Check for HTML
        if content.contains("<html") || content.contains("<body") || content.contains("<!DOCTYPE") {
            return Self::Html;
        }

        // Check for Markdown
        if content.contains("# ") || content.contains("## ") || content.contains("```") {
            return Self::Markdown;
        }

        Self::PlainText
    }
}

/// Parsed document content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedContent {
    /// Original format of the document
    pub format: DocumentFormat,
    /// Cleaned text content
    pub text_content: String,
    /// Structured content (if applicable)
    pub structured_content: Option<StructuredDocument>,
    /// Extracted metadata
    pub metadata: HashMap<String, String>,
    /// Estimated token count
    pub estimated_tokens: Option<i32>,
}

/// Structured document representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDocument {
    /// Document title
    pub title: Option<String>,
    /// Table of contents
    pub toc: Vec<TocEntry>,
    /// Main sections
    pub sections: Vec<DocumentSection>,
    /// Code blocks
    pub code_blocks: Vec<CodeBlock>,
    /// Links and references
    pub links: Vec<Link>,
}

/// Table of contents entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub level: usize,
    pub title: String,
    pub anchor: Option<String>,
}

/// Document section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSection {
    pub level: usize,
    pub title: String,
    pub content: String,
    pub subsections: Vec<DocumentSection>,
}

/// Code block with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
    pub line_start: Option<usize>,
}

/// Link or reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub text: String,
    pub url: String,
    pub title: Option<String>,
}

/// Content chunk for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    /// Chunk content
    pub content: String,
    /// Chunk type (section, paragraph, code block, etc.)
    pub chunk_type: String,
    /// Source document path
    pub source_path: String,
    /// Position in original document
    pub position: usize,
    /// Metadata for retrieval
    pub metadata: HashMap<String, String>,
}

/// Universal parser for multiple document formats
pub struct UniversalParser {
    /// Maximum chunk size in characters
    max_chunk_size: usize,
    /// Overlap between chunks
    #[allow(dead_code)]
    chunk_overlap: usize,
}

impl Default for UniversalParser {
    fn default() -> Self {
        Self {
            max_chunk_size: 2000,
            chunk_overlap: 200,
        }
    }
}

impl UniversalParser {
    /// Create a new universal parser with custom settings
    #[must_use]
    pub fn new(max_chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            max_chunk_size,
            chunk_overlap,
        }
    }

    /// Parse content based on detected format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The content cannot be parsed as the detected format
    /// - The format detection fails
    /// - Required dependencies for parsing are missing
    pub async fn parse(&self, content: &str, path: &str) -> Result<ParsedContent> {
        let format = Self::detect_format(content, path);
        info!("Parsing document: {} (format: {:?})", path, format);

        let parsed = match format {
            DocumentFormat::Markdown => self.parse_markdown(content, path).await?,
            DocumentFormat::Html => self.parse_html(content, path).await?,
            DocumentFormat::Json => self.parse_json(content, path).await?,
            DocumentFormat::Yaml => self.parse_yaml(content, path).await?,
            DocumentFormat::Toml => self.parse_toml(content, path).await?,
            DocumentFormat::Pdf => self.parse_pdf(content, path).await?,
            DocumentFormat::ApiSpec => self.parse_api_spec(content, path).await?,
            DocumentFormat::Code => self.parse_code(content, path).await?,
            DocumentFormat::PlainText => self.parse_plain_text(content, path).await?,
            DocumentFormat::Unknown => self.parse_unknown(content, path).await?,
        };

        Ok(parsed)
    }

    /// Detect document format from content and path
    fn detect_format(content: &str, path: &str) -> DocumentFormat {
        // First try extension-based detection
        let ext_format = DocumentFormat::from_extension(path);
        if !matches!(ext_format, DocumentFormat::Unknown) {
            return ext_format;
        }

        // Fall back to content-based detection
        DocumentFormat::from_content(content)
    }

    /// Parse Markdown content
    #[allow(clippy::unused_async)]
    async fn parse_markdown(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing markdown content from: {}", path);

        // Convert markdown to HTML first
        let html_content = Self::markdown_to_html(content);

        // Extract text from HTML
        let text_content = Self::extract_text_from_html(&html_content);

        // Parse structure
        let structured = Self::parse_markdown_structure(content)?;

        let metadata = HashMap::from([
            ("format".to_string(), "markdown".to_string()),
            (
                "has_code_blocks".to_string(),
                structured.code_blocks.len().to_string(),
            ),
            (
                "sections_count".to_string(),
                structured.sections.len().to_string(),
            ),
        ]);

        let estimated_tokens = Self::estimate_tokens(&text_content);

        Ok(ParsedContent {
            format: DocumentFormat::Markdown,
            text_content,
            structured_content: Some(structured),
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse HTML content
    #[allow(clippy::unused_async)]
    async fn parse_html(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing HTML content from: {}", path);

        let text_content = Self::extract_text_from_html(content);

        // Extract metadata from HTML
        let metadata = Self::extract_html_metadata(content);
        let estimated_tokens = Self::estimate_tokens(&text_content);

        Ok(ParsedContent {
            format: DocumentFormat::Html,
            text_content,
            structured_content: None,
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse JSON content
    #[allow(clippy::unused_async)]
    async fn parse_json(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing JSON content from: {}", path);

        let json_value: Value =
            serde_json::from_str(content).map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

        let text_content = Self::json_to_text(&json_value);
        let metadata = HashMap::from([
            ("format".to_string(), "json".to_string()),
            ("is_valid_json".to_string(), "true".to_string()),
        ]);
        let estimated_tokens = Self::estimate_tokens(&text_content);

        Ok(ParsedContent {
            format: DocumentFormat::Json,
            text_content,
            structured_content: None,
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse YAML content
    #[allow(clippy::unused_async)]
    async fn parse_yaml(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing YAML content from: {}", path);

        // Handle multi-document YAML files (separated by ---)
        let mut combined_text = String::new();
        let mut doc_count = 0;
        
        // Try to parse as multi-document YAML
        for document in serde_yaml::Deserializer::from_str(content) {
            match Value::deserialize(document) {
                Ok(yaml_value) => {
                    doc_count += 1;
                    if doc_count > 1 {
                        combined_text.push_str("\n\n--- Document ");
                        combined_text.push_str(&doc_count.to_string());
                        combined_text.push_str(" ---\n\n");
                    }
                    combined_text.push_str(&Self::yaml_to_text(&yaml_value));
                }
                Err(e) => {
                    // If multi-document parsing fails, try single document
                    if doc_count == 0 {
                        // Fall back to single document parsing
                        let yaml_value: Value = serde_yaml::from_str(content)
                            .map_err(|e| anyhow!("Failed to parse YAML: {}", e))?;
                        combined_text = Self::yaml_to_text(&yaml_value);
                        doc_count = 1;
                        break;
                    }
                    // We already parsed some documents, just log the error and continue
                    warn!("Failed to parse YAML document {} in {}: {}", doc_count + 1, path, e);
                    break;
                }
            }
        }

        let metadata = HashMap::from([
            ("format".to_string(), "yaml".to_string()),
            ("is_valid_yaml".to_string(), "true".to_string()),
            ("document_count".to_string(), doc_count.to_string()),
        ]);
        let estimated_tokens = Self::estimate_tokens(&combined_text);

        Ok(ParsedContent {
            format: DocumentFormat::Yaml,
            text_content: combined_text,
            structured_content: None,
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse TOML content
    #[allow(clippy::unused_async)]
    async fn parse_toml(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing TOML content from: {}", path);

        let toml_value: Value =
            toml::from_str(content).map_err(|e| anyhow!("Failed to parse TOML: {}", e))?;

        let text_content = Self::toml_to_text(&toml_value);
        let metadata = HashMap::from([
            ("format".to_string(), "toml".to_string()),
            ("is_valid_toml".to_string(), "true".to_string()),
        ]);
        let estimated_tokens = Self::estimate_tokens(&text_content);

        Ok(ParsedContent {
            format: DocumentFormat::Toml,
            text_content,
            structured_content: None,
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse PDF content (placeholder for now)
    #[allow(clippy::unused_async)]
    async fn parse_pdf(&self, content: &str, path: &str) -> Result<ParsedContent> {
        warn!("PDF parsing not fully implemented yet for: {}", path);

        let estimated_tokens = Self::estimate_tokens(content);

        Ok(ParsedContent {
            format: DocumentFormat::Pdf,
            text_content: content.to_string(),
            structured_content: None,
            metadata: HashMap::from([
                ("format".to_string(), "pdf".to_string()),
                ("parsing_status".to_string(), "partial".to_string()),
            ]),
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse API specification (`OpenAPI`, GraphQL, etc.)
    #[allow(clippy::unused_async)]
    async fn parse_api_spec(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing API specification from: {}", path);

        let json_value: Value = serde_json::from_str(content)
            .map_err(|e| anyhow!("Failed to parse API spec JSON: {}", e))?;

        let text_content = Self::api_spec_to_text(&json_value);
        let metadata = Self::extract_api_spec_metadata(&json_value);
        let estimated_tokens = Self::estimate_tokens(&text_content);

        Ok(ParsedContent {
            format: DocumentFormat::ApiSpec,
            text_content,
            structured_content: None,
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse code files
    #[allow(clippy::unused_async)]
    async fn parse_code(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing code file from: {}", path);

        let language = Self::detect_code_language(path);
        let metadata = HashMap::from([
            ("format".to_string(), "code".to_string()),
            ("language".to_string(), language.clone()),
            (
                "lines_count".to_string(),
                content.lines().count().to_string(),
            ),
        ]);
        let estimated_tokens = Self::estimate_tokens(content);

        Ok(ParsedContent {
            format: DocumentFormat::Code,
            text_content: content.to_string(),
            structured_content: None,
            metadata,
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse plain text content
    #[allow(clippy::unused_async)]
    async fn parse_plain_text(&self, content: &str, path: &str) -> Result<ParsedContent> {
        debug!("Parsing plain text from: {}", path);

        let estimated_tokens = Self::estimate_tokens(content);

        Ok(ParsedContent {
            format: DocumentFormat::PlainText,
            text_content: content.to_string(),
            structured_content: None,
            metadata: HashMap::from([("format".to_string(), "plain_text".to_string())]),
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Parse unknown format
    #[allow(clippy::unused_async)]
    async fn parse_unknown(&self, content: &str, path: &str) -> Result<ParsedContent> {
        warn!("Parsing unknown format for: {}", path);

        let estimated_tokens = Self::estimate_tokens(content);

        Ok(ParsedContent {
            format: DocumentFormat::Unknown,
            text_content: content.to_string(),
            structured_content: None,
            metadata: HashMap::from([("format".to_string(), "unknown".to_string())]),
            estimated_tokens: Some(estimated_tokens),
        })
    }

    /// Estimate token count (rough approximation: 1 token ≈ 4 characters)
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn estimate_tokens(text: &str) -> i32 {
        (text.len() / 4).max(1) as i32
    }

    /// Parse markdown structure to extract sections and code blocks
    #[allow(clippy::unnecessary_wraps)]
    fn parse_markdown_structure(content: &str) -> Result<StructuredDocument> {
        let mut sections = Vec::new();
        let mut code_blocks = Vec::new();
        let mut toc = Vec::new();
        let mut current_section: Option<DocumentSection> = None;

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check for headers
            if line.starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count();

                if let Some(header_text) = line.get(level..) {
                    let title = header_text.trim().to_string();

                    // Save previous section
                    if let Some(section) = current_section.take() {
                        sections.push(section);
                    }

                    // Create new section
                    current_section = Some(DocumentSection {
                        level,
                        title: title.clone(),
                        content: String::new(),
                        subsections: Vec::new(),
                    });

                    // Add to TOC
                    toc.push(TocEntry {
                        level,
                        title,
                        anchor: None,
                    });
                }
            }
            // Check for code blocks
            else if let Some(language_part) = line.strip_prefix("```") {
                let mut code_content = String::new();
                let language = if language_part.is_empty() {
                    None
                } else {
                    Some(language_part.trim().to_string())
                };

                i += 1; // Skip the opening ```

                while i < lines.len() && !lines[i].starts_with("```") {
                    code_content.push_str(lines[i]);
                    code_content.push('\n');
                    i += 1;
                }

                code_blocks.push(CodeBlock {
                    language,
                    content: code_content.trim().to_string(),
                    line_start: Some(i.saturating_sub(code_content.lines().count())),
                });
            }
            // Add content to current section
            else if let Some(ref mut section) = current_section {
                if !line.trim().is_empty() {
                    section.content.push_str(line);
                    section.content.push('\n');
                }
            }

            i += 1;
        }

        // Save the last section
        if let Some(section) = current_section {
            sections.push(section);
        }

        Ok(StructuredDocument {
            title: Self::extract_title(&sections),
            toc,
            sections,
            code_blocks,
            links: Vec::new(), // Could be enhanced to extract links
        })
    }

    /// Convert markdown to HTML
    fn markdown_to_html(markdown: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(markdown, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    }

    /// Extract text content from HTML
    fn extract_text_from_html(html: &str) -> String {
        let document = Html::parse_document(html);
        let selector = Selector::parse("body").unwrap_or_else(|_| Selector::parse("*").unwrap());

        let mut text_content = String::new();
        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");
            if !text.trim().is_empty() {
                text_content.push_str(&text);
                text_content.push(' ');
            }
        }

        text_content.trim().to_string()
    }

    /// Extract metadata from HTML
    fn extract_html_metadata(html: &str) -> HashMap<String, String> {
        let document = Html::parse_document(html);
        let mut metadata = HashMap::new();

        // Extract title
        if let Ok(title_selector) = Selector::parse("title") {
            for element in document.select(&title_selector) {
                if let Some(title) = element.text().next() {
                    metadata.insert("title".to_string(), title.to_string());
                }
            }
        }

        // Extract meta tags
        if let Ok(meta_selector) = Selector::parse("meta") {
            for element in document.select(&meta_selector) {
                if let (Some(name), Some(content)) = (
                    element
                        .value()
                        .attr("name")
                        .or_else(|| element.value().attr("property")),
                    element.value().attr("content"),
                ) {
                    metadata.insert(name.to_string(), content.to_string());
                }
            }
        }

        metadata.insert("format".to_string(), "html".to_string());
        metadata
    }

    /// Convert JSON value to readable text
    #[allow(clippy::format_push_string)]
    fn json_to_text(value: &Value) -> String {
        match value {
            Value::Object(map) => {
                let mut result = String::new();
                for (key, val) in map {
                    result.push_str(&format!("{}: {}\n", key, Self::json_value_to_string(val)));
                }
                result
            }
            Value::Array(arr) => {
                let mut result = String::new();
                for (i, val) in arr.iter().enumerate() {
                    result.push_str(&format!("{}: {}\n", i, Self::json_value_to_string(val)));
                }
                result
            }
            _ => Self::json_value_to_string(value),
        }
    }

    /// Convert JSON value to string
    fn json_value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Object(_) | Value::Array(_) => format!("{value:?}"),
        }
    }

    /// Convert YAML value to text
    fn yaml_to_text(value: &Value) -> String {
        Self::json_to_text(value) // YAML and JSON have similar structure
    }

    /// Convert TOML value to text
    fn toml_to_text(value: &Value) -> String {
        Self::json_to_text(value) // TOML and JSON have similar structure
    }

    /// Convert API spec to readable text
    #[allow(clippy::format_push_string)]
    fn api_spec_to_text(value: &Value) -> String {
        let mut result = String::new();

        // Extract basic info
        if let Some(title) = value
            .get("info")
            .and_then(|i| i.get("title"))
            .and_then(|t| t.as_str())
        {
            result.push_str(&format!("API Title: {title}\n"));
        }

        if let Some(description) = value
            .get("info")
            .and_then(|i| i.get("description"))
            .and_then(|d| d.as_str())
        {
            result.push_str(&format!("Description: {description}\n"));
        }

        if let Some(version) = value
            .get("info")
            .and_then(|i| i.get("version"))
            .and_then(|v| v.as_str())
        {
            result.push_str(&format!("Version: {version}\n"));
        }

        // Extract paths
        if let Some(paths) = value.get("paths").and_then(|p| p.as_object()) {
            result.push_str("\nAPI Endpoints:\n");
            for (path, methods) in paths {
                result.push_str(&format!("\nPath: {path}\n"));
                if let Some(methods_obj) = methods.as_object() {
                    for (method, details) in methods_obj {
                        if let Some(summary) = details.get("summary").and_then(|s| s.as_str()) {
                            result.push_str(&format!("  {}: {summary}\n", method.to_uppercase()));
                        }
                    }
                }
            }
        }

        result
    }

    /// Extract API spec metadata
    fn extract_api_spec_metadata(value: &Value) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        metadata.insert("format".to_string(), "api_spec".to_string());

        if let Some(spec_version) = value
            .get("openapi")
            .or_else(|| value.get("swagger"))
            .and_then(|v| v.as_str())
        {
            metadata.insert("spec_version".to_string(), spec_version.to_string());
        }

        if let Some(title) = value
            .get("info")
            .and_then(|i| i.get("title"))
            .and_then(|t| t.as_str())
        {
            metadata.insert("title".to_string(), title.to_string());
        }

        if let Some(paths) = value.get("paths").and_then(|p| p.as_object()) {
            metadata.insert("endpoint_count".to_string(), paths.len().to_string());
        }

        metadata
    }

    /// Detect programming language from file path
    fn detect_code_language(path: &str) -> String {
        let path_obj = Path::new(path);
        if let Some(ext) = path_obj.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "rs" => "rust".to_string(),
                "py" => "python".to_string(),
                "js" => "javascript".to_string(),
                "ts" => "typescript".to_string(),
                "go" => "go".to_string(),
                "java" => "java".to_string(),
                "cpp" | "cc" | "cxx" => "cpp".to_string(),
                "c" => "c".to_string(),
                "h" => "c_header".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }

    /// Extract title from document sections
    fn extract_title(sections: &[DocumentSection]) -> Option<String> {
        // Look for the first top-level section (level 1)
        for section in sections {
            if section.level == 1 {
                return Some(section.title.clone());
            }
        }

        // If no level 1 section, look for level 2
        for section in sections {
            if section.level == 2 {
                return Some(section.title.clone());
            }
        }

        None
    }

    /// Chunk content into smaller pieces for embedding
    #[must_use]
    pub fn chunk_content(&self, parsed: &ParsedContent, source_path: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        // If we have structured content, use it for intelligent chunking
        if let Some(structured) = &parsed.structured_content {
            for section in &structured.sections {
                let chunk = ContentChunk {
                    content: format!("{}\n\n{}", section.title, section.content),
                    chunk_type: "section".to_string(),
                    source_path: source_path.to_string(),
                    position: 0, // Could be enhanced with actual position
                    metadata: HashMap::from([
                        ("section_level".to_string(), section.level.to_string()),
                        ("section_title".to_string(), section.title.clone()),
                    ]),
                };
                chunks.push(chunk);
            }

            // Add code blocks as separate chunks
            for code_block in &structured.code_blocks {
                let chunk = ContentChunk {
                    content: code_block.content.clone(),
                    chunk_type: "code_block".to_string(),
                    source_path: source_path.to_string(),
                    position: code_block.line_start.unwrap_or(0),
                    metadata: HashMap::from([
                        (
                            "language".to_string(),
                            code_block
                                .language
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                        ),
                        ("is_code".to_string(), "true".to_string()),
                    ]),
                };
                chunks.push(chunk);
            }
        } else {
            // Fallback to simple text chunking
            chunks.extend(self.chunk_text(&parsed.text_content, source_path));
        }

        chunks
    }

    /// Simple text chunking for unstructured content
    fn chunk_text(&self, text: &str, source_path: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return chunks;
        }

        let mut current_chunk = String::new();
        let mut position = 0;

        for &word in &words {
            if current_chunk.len() + word.len() + 1 > self.max_chunk_size {
                if !current_chunk.is_empty() {
                    chunks.push(ContentChunk {
                        content: current_chunk.trim().to_string(),
                        chunk_type: "paragraph".to_string(),
                        source_path: source_path.to_string(),
                        position,
                        metadata: HashMap::new(),
                    });
                    position += current_chunk.len();
                }
                current_chunk = word.to_string();
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(word);
            }
        }

        // Add the last chunk
        if !current_chunk.is_empty() {
            chunks.push(ContentChunk {
                content: current_chunk.trim().to_string(),
                chunk_type: "paragraph".to_string(),
                source_path: source_path.to_string(),
                position,
                metadata: HashMap::new(),
            });
        }

        chunks
    }
}

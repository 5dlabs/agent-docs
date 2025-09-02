//! Intelligent document discovery and ingestion
//!
//! This module provides AI-powered document discovery and classification
//! using Claude to intelligently identify and extract relevant documentation
//! from various sources including GitHub repositories, websites, and APIs.

use anyhow::{anyhow, Result};
use chrono::Utc;
use octocrab::Octocrab;


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use tracing::{info, warn};
use url::Url;

use crate::loaders::{DocPage, RateLimiter};
use crate::parsers::UniversalParser;

/// Document source types for intelligent discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentSource {
    /// GitHub repository with owner/repo format
    GitHubRepo { owner: String, repo: String },
    /// Specific GitHub file with full URL
    GitHubFile { url: String, path: String },
    /// Web page with URL
    WebPage { url: String },
    /// API documentation with base URL and optional spec URL
    ApiDocs {
        base_url: String,
        spec_url: Option<String>,
    },
    /// Local file path
    LocalFile { path: PathBuf },
    /// Raw markdown content
    RawMarkdown { content: String, source: String },
}

impl DocumentSource {
    /// Get the URL representation of this source
    #[must_use]
    pub fn url(&self) -> String {
        match self {
            DocumentSource::GitHubRepo { owner, repo } => {
                format!("https://github.com/{owner}/{repo}")
            }
            DocumentSource::GitHubFile { url, .. } | DocumentSource::WebPage { url } => url.clone(),
            DocumentSource::ApiDocs { base_url, .. } => base_url.clone(),
            DocumentSource::LocalFile { path } => format!("file://{}", path.display()),
            DocumentSource::RawMarkdown { source, .. } => source.clone(),
        }
    }

    /// Get the display name for this source
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            DocumentSource::GitHubRepo { owner, repo } => format!("{owner}/{repo}"),
            DocumentSource::GitHubFile { path, .. } => path.clone(),
            DocumentSource::WebPage { url } => url.clone(),
            DocumentSource::ApiDocs { base_url, .. } => base_url.clone(),
            DocumentSource::LocalFile { path } => path.display().to_string(),
            DocumentSource::RawMarkdown { source, .. } => source.clone(),
        }
    }
}

/// Analysis result from Claude's repository analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Overall documentation quality score (0-10)
    pub quality_score: f32,
    /// List of identified documentation files with priorities
    pub docs_found: Vec<DocFile>,
    /// Recommended ingestion strategy
    pub strategy: IngestionStrategy,
    /// Additional metadata from analysis
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Individual documentation file found during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocFile {
    /// File path or URL
    pub path: String,
    /// Priority score (0-10, higher is more important)
    pub priority: f32,
    /// Type of documentation
    pub doc_type: String,
    /// Estimated content size in tokens
    pub estimated_tokens: Option<i32>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Recommended ingestion strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionStrategy {
    /// Whether to use AI for content extraction
    pub use_ai_extraction: bool,
    /// Batch size for processing
    pub batch_size: usize,
    /// Whether to include code examples
    pub include_examples: bool,
    /// Whether to include API references
    pub include_api_refs: bool,
    /// Additional processing instructions
    pub instructions: Vec<String>,
}

/// Core trait for intelligent document discovery and extraction
#[async_trait::async_trait]
pub trait IntelligentLoader {
    /// Discover all relevant documents from a given URL
    async fn discover_documents(&mut self, url: &str) -> Result<Vec<DocumentSource>>;

    /// Classify content and determine optimal parsing strategy
    async fn classify_content(&self, content: &str) -> Result<DocumentType>;

    /// Extract relevant documents from a discovered source
    async fn extract_relevant(&mut self, source: DocumentSource) -> Result<Vec<DocPage>>;
}

/// Claude-powered intelligent loader implementation
pub struct ClaudeIntelligentLoader {
    /// Claude LLM client
    llm_client: llm::client::LlmClient,
    /// Rate limiter for API calls
    rate_limiter: RateLimiter,
    /// GitHub API client
    github_client: Octocrab,
    /// Universal parser for multiple document formats
    parser: UniversalParser,
}

impl ClaudeIntelligentLoader {
    /// Create a new Claude-powered intelligent loader
    ///
    /// # Panics
    ///
    /// Panics if the GitHub client cannot be created.
    #[must_use]
    pub fn new() -> Self {
        Self {
            llm_client: llm::client::LlmClient::new(),
            rate_limiter: RateLimiter::new(),
            github_client: Octocrab::builder().build().unwrap_or_else(|_| {
                warn!("Failed to create GitHub client, using anonymous access");
                Octocrab::default()
            }),
            parser: UniversalParser::new(2000, 200),
        }
    }

    /// Analyze a GitHub repository using Claude
    ///
    /// # Errors
    ///
    /// Returns an error if the repository URL is invalid, the GitHub API call fails,
    /// or Claude analysis fails.
    pub async fn analyze_repository(&self, repo_url: &str) -> Result<AnalysisResult> {
        info!("Analyzing repository: {}", repo_url);

        // Parse repository URL
        let (owner, repo) = Self::parse_github_url(repo_url)?;

        // Get repository structure
        let repo_info = self.github_client.repos(&owner, &repo).get().await?;
        let repo_tree = self.get_repository_tree(&owner, &repo).await?;

        // Use Claude to analyze the repository
        let analysis_prompt = Self::build_analysis_prompt(&repo_info, &repo_tree);
        let analysis_response = self.llm_client.summarize(&analysis_prompt)?;

        // Parse Claude's response
        Self::parse_analysis_response(&analysis_response)
    }

    /// Build the analysis prompt for Claude
    fn build_analysis_prompt(
        repo_info: &octocrab::models::Repository,
        repo_tree: &[String],
    ) -> String {
        format!(
            r#"Analyze this GitHub repository and identify all relevant documentation:

Repository: {}/{}
Description: {}
Topics: {}
Language: {}
Stars: {}
Last Updated: {}

Repository Structure:
{}

Please analyze this repository and provide:
1. Overall documentation quality score (0-10)
2. List of documentation files with priorities
3. Recommended ingestion strategy

Focus on:
- README files and documentation directories
- API documentation and specifications
- Configuration guides and examples
- Architecture and design documents
- Code comments and inline documentation

Return your analysis in JSON format with the following structure:
{{
    "quality_score": <float>,
    "docs_found": [
        {{
            "path": "<file_path>",
            "priority": <float>,
            "doc_type": "<type>",
            "estimated_tokens": <int>,
            "metadata": {{}}
        }}
    ],
    "strategy": {{
        "use_ai_extraction": <boolean>,
        "batch_size": <int>,
        "include_examples": <boolean>,
        "include_api_refs": <boolean>,
        "instructions": ["<instruction1>", "<instruction2>"]
    }},
    "metadata": {{}}
}}"#,
            repo_info.owner.clone().unwrap().login,
            repo_info.name.clone(),
            repo_info.description.as_deref().unwrap_or("No description"),
            repo_info.topics.clone().unwrap_or_default().join(", "),
            repo_info
                .language
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown"),
            repo_info.stargazers_count.unwrap_or(0),
            repo_info.updated_at.map_or("Unknown".to_string(), |dt| dt
                .format("%Y-%m-%d")
                .to_string()),
            repo_tree.join("\n")
        )
    }

    /// Parse Claude's analysis response
    fn parse_analysis_response(response: &str) -> Result<AnalysisResult> {
        // Try to extract JSON from Claude's response
        let json_start = response
            .find('{')
            .ok_or_else(|| anyhow!("No JSON found in response"))?;
        let json_content = &response[json_start..];

        let analysis: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| anyhow!("Failed to parse Claude analysis: {}", e))?;

        #[allow(clippy::cast_possible_truncation)]
        let quality_score = analysis["quality_score"]
            .as_f64()
            .ok_or_else(|| anyhow!("Missing quality_score"))? as f32;

        let docs_found = analysis["docs_found"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing docs_found"))?
            .iter()
            .filter_map(|doc| {
                Some(DocFile {
                    path: doc["path"].as_str()?.to_string(),
                    #[allow(clippy::cast_possible_truncation)]
                    priority: doc["priority"].as_f64()? as f32,
                    doc_type: doc["doc_type"].as_str()?.to_string(),
                    #[allow(clippy::cast_possible_truncation)]
                    estimated_tokens: doc["estimated_tokens"].as_i64().map(|v| v as i32),
                    metadata: doc["metadata"]
                        .as_object()?
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect(),
                })
            })
            .collect();

        let strategy = IngestionStrategy {
            use_ai_extraction: analysis["strategy"]["use_ai_extraction"]
                .as_bool()
                .unwrap_or(true),
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            batch_size: analysis["strategy"]["batch_size"].as_i64().unwrap_or(10) as usize,
            include_examples: analysis["strategy"]["include_examples"]
                .as_bool()
                .unwrap_or(true),
            include_api_refs: analysis["strategy"]["include_api_refs"]
                .as_bool()
                .unwrap_or(true),
            instructions: analysis["strategy"]["instructions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|inst| inst.as_str().map(ToString::to_string))
                .collect(),
        };

        let metadata = analysis["metadata"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(AnalysisResult {
            quality_score,
            docs_found,
            strategy,
            metadata,
        })
    }

    /// Parse GitHub URL to extract owner and repository
    fn parse_github_url(url: &str) -> Result<(String, String)> {
        let parsed_url = Url::parse(url)?;
        let path_segments: Vec<&str> = parsed_url
            .path_segments()
            .map(Iterator::collect)
            .unwrap_or_default();

        if path_segments.len() >= 2 {
            Ok((path_segments[0].to_string(), path_segments[1].to_string()))
        } else {
            Err(anyhow!("Invalid GitHub URL format"))
        }
    }

    /// Get repository file tree
    async fn get_repository_tree(&self, owner: &str, repo: &str) -> Result<Vec<String>> {
        let contents = self
            .github_client
            .repos(owner, repo)
            .get_content()
            .path("")
            .r#ref("HEAD")
            .send()
            .await?;

        let mut tree = Vec::new();
        for item in contents.items {
            if item.r#type == "file" {
                tree.push(item.path);
            }
        }

        Ok(tree)
    }




}

impl Default for ClaudeIntelligentLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl IntelligentLoader for ClaudeIntelligentLoader {
    async fn discover_documents(&mut self, url: &str) -> Result<Vec<DocumentSource>> {
        info!("Discovering documents from: {}", url);

        if url.contains("github.com") {
            // Handle GitHub repositories
            let (owner, repo) = Self::parse_github_url(url)?;
            let analysis = self.analyze_repository(url).await?;

            let mut sources = Vec::new();

            // Add high-priority documentation files
            for doc_file in analysis.docs_found {
                if doc_file.priority >= 5.0 {
                    sources.push(DocumentSource::GitHubFile {
                        url: format!(
                            "https://raw.githubusercontent.com/{owner}/{repo}/HEAD/{path}",
                            path = doc_file.path
                        ),
                        path: doc_file.path,
                    });
                }
            }

            // Always include README if not already found
            if !sources.iter().any(
                |s| matches!(s, DocumentSource::GitHubFile { path, .. } if path.contains("README")),
            ) {
                sources.push(DocumentSource::GitHubFile {
                    url: format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/README.md"),
                    path: "README.md".to_string(),
                });
            }

            Ok(sources)
        } else if url.starts_with("http") {
            // Handle web pages
            Ok(vec![DocumentSource::WebPage {
                url: url.to_string(),
            }])
        } else {
            Err(anyhow!("Unsupported URL format: {}", url))
        }
    }

    async fn classify_content(&self, content: &str) -> Result<DocumentType> {
        // Use Claude to classify the content type
        let classification_prompt = format!(
            r#"Classify this documentation content and determine the best parsing strategy:

Content Preview:
{}

Please classify this content as one of the following types:
- markdown: Standard markdown documentation
- api_spec: API specification (OpenAPI, GraphQL, etc.)
- guide: User guide or tutorial
- reference: API reference documentation
- config: Configuration files or documentation
- code: Source code with documentation
- other: Other types of documentation

Return your classification in JSON format:
{{
    "doc_type": "<type>",
    "confidence": <float>,
    "parsing_strategy": "<strategy>",
    "metadata": {{}}
}}"#,
            &content[..content.len().min(1000)]
        );

        let response = self.llm_client.summarize(&classification_prompt)?;
        let classification: serde_json::Value = serde_json::from_str(&response)?;

        match classification["doc_type"].as_str() {
            Some("markdown") => Ok(DocumentType::Markdown),
            Some("api_spec") => Ok(DocumentType::ApiSpec),
            Some("guide") => Ok(DocumentType::Guide),
            Some("reference") => Ok(DocumentType::Reference),
            Some("config") => Ok(DocumentType::Config),
            Some("code") => Ok(DocumentType::Code),
            _ => Ok(DocumentType::Other),
        }
    }

    async fn extract_relevant(&mut self, source: DocumentSource) -> Result<Vec<DocPage>> {
        match source {
            DocumentSource::GitHubFile { url, path } => self.extract_github_file(&url, &path).await,
            DocumentSource::WebPage { url } => self.extract_web_page(&url).await,
            DocumentSource::GitHubRepo { owner, repo } => {
                self.extract_repo_readme(&owner, &repo).await
            }
            DocumentSource::LocalFile { path } => self.extract_local_file(&path).await,
            _ => Err(anyhow!("Unsupported source type for extraction")),
        }
    }
}

/// Document type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    Markdown,
    ApiSpec,
    Guide,
    Reference,
    Config,
    Code,
    Other,
}

impl ClaudeIntelligentLoader {
    /// Extract content from a GitHub file
    async fn extract_github_file(&mut self, url: &str, path: &str) -> Result<Vec<DocPage>> {
        let response = self.rate_limiter.get(url).await?;
        let content = response.text().await?;

        // Use UniversalParser to parse the content
        let parsed = self.parser.parse(&content, path).await?;

        let doc_page = DocPage {
            url: url.to_string(),
            content: parsed.text_content,
            item_type: Self::determine_file_type(path),
            module_path: path.to_string(),
            extracted_at: Utc::now(),
        };

        Ok(vec![doc_page])
    }

    /// Extract content from a web page
    async fn extract_web_page(&mut self, url: &str) -> Result<Vec<DocPage>> {
        let response = self.rate_limiter.get(url).await?;
        let html_content = response.text().await?;

        // Use UniversalParser to parse the HTML
        let parsed = self.parser.parse(&html_content, url).await?;

        let doc_page = DocPage {
            url: url.to_string(),
            content: parsed.text_content,
            item_type: "web_page".to_string(),
            module_path: Url::parse(url)?.path().to_string(),
            extracted_at: Utc::now(),
        };

        Ok(vec![doc_page])
    }

    /// Extract content from a local file
    async fn extract_local_file(&mut self, path: &PathBuf) -> Result<Vec<DocPage>> {
        use tokio::fs;

        let content = fs::read_to_string(path).await
            .map_err(|e| anyhow!("Failed to read local file {}: {}", path.display(), e))?;

        // Use UniversalParser to parse the content
        let path_str = path.to_string_lossy();
        let parsed = self.parser.parse(&content, &path_str).await?;

        let doc_page = DocPage {
            url: format!("file://{}", path.display()),
            content: parsed.text_content,
            item_type: Self::determine_file_type(&path_str),
            module_path: path_str.to_string(),
            extracted_at: Utc::now(),
        };

        Ok(vec![doc_page])
    }

    /// Extract README from a GitHub repository
    async fn extract_repo_readme(&mut self, owner: &str, repo: &str) -> Result<Vec<DocPage>> {
        let readme_url = format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/README.md");
        self.extract_github_file(&readme_url, "README.md").await
    }

    /// Determine file type from path
    fn determine_file_type(path: &str) -> String {
        let path_obj = std::path::Path::new(path);
        if let Some(ext) = path_obj.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "md" | "markdown" => "markdown".to_string(),
                "rs" => "rust_source".to_string(),
                "json" => "json_config".to_string(),
                "yaml" | "yml" => "yaml_config".to_string(),
                "toml" => "toml_config".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }
}

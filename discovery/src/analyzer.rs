use crate::claude::ClaudeRunner;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryAnalysis {
    pub repo_info: RepoInfo,
    pub strategy: IngestionStrategy,
    pub cli_commands: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoInfo {
    pub url: String,
    pub name: String,
    pub primary_language: Option<String>,
    pub documentation_type: DocumentationType,
    pub estimated_size: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DocumentationType {
    Software,
    Api,
    Tutorial,
    Reference,
    Mixed,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IngestionStrategy {
    pub docs_only: bool,
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub extensions: Vec<String>,
    pub recursive: bool,
    pub chunk_size: Option<usize>,
    pub use_ai_chunking: bool,
    pub doc_type: String,
    pub source_name: String,
}

pub struct IntelligentRepositoryAnalyzer {
    runner: ClaudeRunner,
}

impl IntelligentRepositoryAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            runner: ClaudeRunner::new(),
        }
    }

    /// Analyze a GitHub repository and produce an ingest plan.
    ///
    /// # Errors
    /// Returns an error if the Claude runner fails or the response cannot be parsed.
    pub async fn analyze_repository(&mut self, github_url: &str) -> Result<RepositoryAnalysis> {
        info!("Analyzing repository (discovery): {}", github_url);
        let repo_info = Self::get_repository_info(github_url)?;
        let prompt = Self::create_analysis_prompt(github_url, &repo_info);
        let raw = self.runner.run(&prompt).await?;
        Self::parse_claude_analysis(&raw, &repo_info)
    }

    fn get_repository_info(github_url: &str) -> Result<RepoInfo> {
        let (_, repo_name) = Self::parse_github_url(github_url)?;
        Ok(RepoInfo {
            url: github_url.to_string(),
            name: repo_name,
            primary_language: None,
            documentation_type: DocumentationType::Unknown,
            estimated_size: String::new(),
        })
    }

    fn parse_github_url(url_str: &str) -> Result<(String, String)> {
        let url = url_str.trim_end_matches('/');
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 2 {
            return Err(anyhow!("Invalid GitHub URL format"));
        }
        Ok((
            parts[parts.len() - 2].to_string(),
            parts[parts.len() - 1].to_string(),
        ))
    }

    fn create_analysis_prompt(github_url: &str, repo_info: &RepoInfo) -> String {
        let mut structure = String::new();
        let _ = writeln!(structure, "Key documentation files found:");
        format!(
            r#"
TASK: Analyze the GitHub repository and create a comprehensive documentation ingestion strategy.

REPOSITORY TO ANALYZE: {}
REPOSITORY NAME: {}

ACTUAL REPOSITORY STRUCTURE:
{}

CRITICAL: You MUST respond with valid JSON in exactly this format. Prefer widely compatible commands and include multiple loader cli steps if docs may be in different locations.

{{
    "repository_type": "network-security|rust-library|api-docs|tutorial|reference|mixed",
    "documentation_assessment": {{
        "primary_format": "markdown|rst|html|mixed",
        "key_directories": ["Documentation/", "docs/", "api/"],
        "estimated_file_count": 500,
        "complexity": "simple|moderate|complex",
        "has_api_docs": true,
        "has_tutorials": true,
        "documentation_quality": "excellent|good|fair|poor"
    }},
    "ingestion_strategy": {{
        "docs_only": true,
        "extensions": ["<choose only relevant formats>", "md", "mdx", "rst", "html"],
        "recursive": true,
        "chunk_size": 2000,
        "use_ai_chunking": true,
        "doc_type": "example",
        "source_name": "github"
    }},
    "cli_commands": [
        "git clone --depth 1 REPO_URL UNIQUE_REPO_DIR",
        "loader cli UNIQUE_REPO_DIR --extensions md,html --recursive -o UNIQUE_DOCS_OUT",
        "loader database --input-dir UNIQUE_DOCS_OUT --doc-type example --source-name github --yes"
    ],
    "reasoning": "Detailed explanation of the strategy and decisions."
}}

DISCOVERY HINTS (consider these common layouts when picking include_paths and formats):
- docs/
- Documentation/
- doc/
- website/docs
- website/content/docs (Hugo)
- website/content
- docs/source (Sphinx)
- content/docs
- docs/content
Choose the minimal set of file formats that carry documentation value for this repo. Prefer md/mdx/rst/html. Include yaml/yml/toml/json only if they contain human-facing docs (not just config or generated schemas). Exclude build output, vendored libs, and test fixtures.

IMPORTANT CONSTRAINTS FOR cli_commands:
1. The 'loader cli' command ONLY accepts these arguments:
   - PATH (required): The directory or file to process
   - --extensions: Comma-separated list of file extensions (e.g., md,html,yaml)
   - --recursive: Flag to traverse directories recursively
   - -o or --output: Output directory path
   
2. DO NOT use any of these invalid flags with 'loader cli':
   - --include-dirs (NOT VALID)
   - --exclude-dirs (NOT VALID)
   - --include-paths (NOT VALID)
   - --exclude-paths (NOT VALID)
   - Any other flags not listed above

3. To process specific directories, use the PATH argument directly:
   - CORRECT: "loader cli UNIQUE_REPO_DIR/docs --extensions md,mdx,rst,html,json,yaml,yml,toml,txt --recursive -o UNIQUE_DOCS_OUT"
   - WRONG: "loader cli UNIQUE_REPO_DIR --include-dirs docs --extensions md --recursive -o UNIQUE_DOCS_OUT"

4. If unsure of the exact path, include multiple 'loader cli' commands, each targeting a different common docs directory (e.g., docs/, website/content/docs, docs/source).
   - CORRECT: "loader cli UNIQUE_REPO_DIR/website/content/docs --extensions md,mdx,rst,html,json,yaml,yml,toml,txt --recursive -o UNIQUE_DOCS_OUT"
   - CORRECT: "loader cli UNIQUE_REPO_DIR/docs/source --extensions md,mdx,rst,html,json,yaml,yml,toml,txt --recursive -o UNIQUE_DOCS_OUT"

RESPOND ONLY WITH THE JSON. DO NOT include any other text before or after the JSON.
"#,
            github_url, repo_info.name, structure
        )
    }

    #[allow(clippy::too_many_lines)]
    fn parse_claude_analysis(
        claude_response: &str,
        repo_info: &RepoInfo,
    ) -> Result<RepositoryAnalysis> {
        // Try stream-json first, else slice braces
        let mut json_content: Option<String> = None;
        for line in claude_response.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(t) = val.get("type").and_then(|v| v.as_str()) {
                    match t {
                        "assistant" => {
                            if let Some(arr) = val
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_array())
                            {
                                for item in arr {
                                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                        if text.contains('{') {
                                            if let (Some(s), Some(e)) =
                                                (text.find('{'), text.rfind('}'))
                                            {
                                                json_content = Some(text[s..=e].to_string());
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        "result" => {
                            if let Some(text) = val.get("result").and_then(|v| v.as_str()) {
                                if let (Some(s), Some(e)) = (text.find('{'), text.rfind('}')) {
                                    json_content = Some(text[s..=e].to_string());
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let json_str = json_content.unwrap_or_else(|| {
            let s = claude_response.find('{').unwrap_or(0);
            let e = claude_response
                .rfind('}')
                .unwrap_or(claude_response.len() - 1);
            claude_response[s..=e].to_string()
        });

        let v: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse Claude response JSON: {}", e))?;

        let strategy_json = v
            .get("ingestion_strategy")
            .ok_or_else(|| anyhow!("missing ingestion_strategy"))?;
        let strategy = IngestionStrategy {
            docs_only: strategy_json
                .get("docs_only")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            include_paths: strategy_json
                .get("include_paths")
                .and_then(serde_json::Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|e| e.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            exclude_paths: strategy_json
                .get("exclude_paths")
                .and_then(serde_json::Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|e| e.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            extensions: strategy_json
                .get("extensions")
                .and_then(serde_json::Value::as_array)
                .map_or_else(
                    || vec!["md".to_string(), "rst".to_string()],
                    |a| {
                        a.iter()
                            .filter_map(|e| e.as_str().map(ToString::to_string))
                            .collect()
                    },
                ),
            recursive: strategy_json
                .get("recursive")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            chunk_size: strategy_json
                .get("chunk_size")
                .and_then(serde_json::Value::as_u64)
                .map(|u| usize::try_from(u).unwrap_or(1000)),
            use_ai_chunking: strategy_json
                .get("use_ai_chunking")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            doc_type: strategy_json
                .get("doc_type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            source_name: strategy_json
                .get("source_name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("github-repo")
                .to_string(),
        };
        let cli_commands = v
            .get("cli_commands")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|e| e.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let reasoning = v
            .get("reasoning")
            .and_then(|x| x.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        Ok(RepositoryAnalysis {
            repo_info: repo_info.clone(),
            strategy,
            cli_commands,
            reasoning,
        })
    }
}

impl Default for IntelligentRepositoryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

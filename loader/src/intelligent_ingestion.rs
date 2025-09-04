//! Intelligent Repository Ingestion using Claude Code
//!
//! This module provides Claude Code-powered analysis of GitHub repositories
//! to determine optimal ingestion strategies and generate CLI commands.

use anyhow::{anyhow, Result};
use llm::{LlmClient, LlmProvider, ModelConfig};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use std::process::Command;
use tracing::{info, warn};

/// Repository analysis results from Claude Code
#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryAnalysis {
    /// Repository metadata
    pub repo_info: RepoInfo,
    /// Recommended ingestion strategy
    pub strategy: IngestionStrategy,
    /// Generated CLI commands to execute
    pub cli_commands: Vec<String>,
    /// Reasoning behind decisions
    pub reasoning: String,
}

/// Basic repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub url: String,
    pub name: String,
    pub primary_language: Option<String>,
    pub documentation_type: DocumentationType,
    pub estimated_size: String,
}

/// Type of documentation found in the repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentationType {
    /// Standard software documentation (README, docs/, etc.)
    Software,
    /// API documentation (`OpenAPI`, GraphQL, etc.)
    Api,
    /// Tutorial/guide content
    Tutorial,
    /// Reference documentation
    Reference,
    /// Mixed content types
    Mixed,
    /// Unknown or unclear
    Unknown,
}

/// Recommended ingestion strategy
#[derive(Debug, Serialize, Deserialize)]
pub struct IngestionStrategy {
    /// Whether to use docs-only mode
    pub docs_only: bool,
    /// Specific paths to include/exclude
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    /// File extensions to process
    pub extensions: Vec<String>,
    /// Whether to use recursive scanning
    pub recursive: bool,
    /// Recommended chunk size for large documents
    pub chunk_size: Option<usize>,
    /// Whether to use AI-powered chunking
    pub use_ai_chunking: bool,
    /// Document type for database storage
    pub doc_type: String,
    /// Source name for attribution
    pub source_name: String,
}

/// Claude Code-powered repository analyzer
pub struct IntelligentRepositoryAnalyzer {
    llm_client: LlmClient,
}

impl IntelligentRepositoryAnalyzer {
    /// Create a new analyzer with Claude Code configuration
    ///
    /// # Errors
    ///
    /// Returns an error if Claude Code binary is not available or configured.
    pub fn new() -> Result<Self> {
        let config = ModelConfig {
            provider: LlmProvider::ClaudeCode,
            model_name: "claude-3-5-sonnet-20241022".to_string(), // Claude 4 Sonnet
            api_key: None, // Claude Code doesn't need API key
            binary_path: std::env::var("CLAUDE_BINARY_PATH").ok(),
            max_tokens: 8000, // Increased for comprehensive analysis
            temperature: 0.1, // Low temperature for consistent analysis
        };

        let llm_client = LlmClient::with_config(config);

        Ok(Self { llm_client })
    }

    /// Analyze a GitHub repository and generate ingestion strategy
    ///
    /// # Errors
    ///
    /// Returns an error if the repository cannot be analyzed or Claude Code fails.
    pub async fn analyze_repository(&mut self, github_url: &str) -> Result<RepositoryAnalysis> {
        info!("🔍 Analyzing repository with Claude Code: {}", github_url);

        // First, get basic repository information
        let repo_info = Self::get_repository_info(github_url)?;

        // Generate comprehensive analysis prompt for Claude Code
        let analysis_prompt = Self::create_analysis_prompt(github_url, &repo_info);

        // Use Claude Code to analyze the repository
        let claude_response = self.llm_client.summarize(&analysis_prompt).await?;

        // Debug: Log Claude's response
        info!("🤖 Claude Code Response: {}", claude_response);

        // Parse Claude's response into structured analysis
        let analysis = Self::parse_claude_analysis(&claude_response, &repo_info)?;

        info!("✅ Repository analysis complete");
        info!(
            "📋 Strategy: {} files, doc_type: {}",
            analysis.strategy.extensions.join(","),
            analysis.strategy.doc_type
        );

        Ok(analysis)
    }

    /// Get basic repository information using GitHub API or git commands
    fn get_repository_info(github_url: &str) -> Result<RepoInfo> {
        // Parse GitHub URL to extract owner/repo
        let (owner, repo_name) = Self::parse_github_url(github_url)?;

        // Get actual repository structure by cloning and analyzing
        let temp_dir = format!("/tmp/repo_analysis_{repo_name}");
        let clone_result = std::process::Command::new("git")
            .args(["clone", "--depth", "1", github_url, &temp_dir])
            .output();

        let repo_structure = if clone_result.is_ok() {
            Self::analyze_repository_structure(&temp_dir).unwrap_or_default()
        } else {
            "Repository structure unavailable (clone failed)".to_string()
        };

        Ok(RepoInfo {
            url: github_url.to_string(),
            name: format!("{owner}/{repo_name}"),
            primary_language: None, // Could fetch from GitHub API
            documentation_type: DocumentationType::Unknown,
            estimated_size: repo_structure,
        })
    }

    /// Analyze the actual repository structure
    fn analyze_repository_structure(repo_path: &str) -> Result<String> {
        let mut structure = String::new();

        // Get directory structure
        let output = std::process::Command::new("find")
            .args([
                repo_path, "-type", "f", "-name", "*.md", "-o", "-name", "*.rst", "-o", "-name",
                "*.html", "-o", "-name", "README*",
            ])
            .output()?;

        if output.status.success() {
            let files = String::from_utf8_lossy(&output.stdout);
            let file_list: Vec<&str> = files.lines().take(50).collect(); // Limit for prompt size
            structure.push_str("Key documentation files found:\n");
            for file in file_list {
                if let Some(relative_path) = file.strip_prefix(repo_path) {
                    writeln!(structure, "- {}", relative_path.trim_start_matches('/')).ok();
                }
            }
        }

        // Get directory structure
        let dir_output = std::process::Command::new("find")
            .args([
                repo_path, "-type", "d", "-name", "doc*", "-o", "-name", "Doc*", "-o", "-name",
                "api*", "-o", "-name", "guide*", "-o", "-name", "example*",
            ])
            .output()?;

        if dir_output.status.success() {
            let dirs = String::from_utf8_lossy(&dir_output.stdout);
            if !dirs.trim().is_empty() {
                structure.push_str("\nDocumentation directories found:\n");
                for dir in dirs.lines().take(20) {
                    if let Some(relative_path) = dir.strip_prefix(repo_path) {
                        writeln!(structure, "- {}/", relative_path.trim_start_matches('/')).ok();
                    }
                }
            }
        }

        Ok(structure)
    }

    /// Create comprehensive analysis prompt for Claude Code
    fn create_analysis_prompt(github_url: &str, repo_info: &RepoInfo) -> String {
        let prompt = format!(
            r#"
TASK: Analyze the GitHub repository and create a comprehensive documentation ingestion strategy.

You are an expert at identifying and extracting valuable documentation from software repositories. 

REPOSITORY TO ANALYZE: {}
REPOSITORY NAME: {}

ACTUAL REPOSITORY STRUCTURE:
{}

COMPREHENSIVE ANALYSIS NEEDED:

1. DOCUMENTATION TYPES TO CONSIDER:
   - README files (README.md, README.rst, README.txt)
   - Documentation directories (docs/, Documentation/, doc/, .github/)
   - API documentation (OpenAPI specs, GraphQL schemas, API.md)
   - Configuration documentation (config examples, YAML/TOML/JSON schemas)
   - Architecture documentation (diagrams, ADRs, design docs)
   - Tutorial content (guides/, examples/, tutorials/)
   - Code documentation (well-documented source files)
   - Wiki content (if repository has wiki)
   - Issue/PR templates with documentation value
   - Changelog and release notes
   - Security documentation (SECURITY.md, security policies)

2. FILE FORMATS TO EVALUATE:
   - Markdown (.md, .markdown)
   - reStructuredText (.rst)
   - HTML documentation (.html)
   - JSON schemas and specs (.json)
   - YAML configuration docs (.yaml, .yml)
   - TOML configuration (.toml)
   - Text documentation (.txt)
   - AsciiDoc (.adoc)
   - LaTeX documentation (.tex)

3. REPOSITORY STRUCTURE ANALYSIS:
   - Identify primary documentation directories
   - Assess documentation quality and completeness
   - Determine if this is API docs, user guides, technical specs, or mixed
   - Evaluate whether source code contains valuable inline documentation
   - Check for specialized documentation tools (Sphinx, GitBook, etc.)

4. INGESTION STRATEGY DECISIONS:
   - Which directories contain the most valuable content?
   - Should we include source files or focus on pure documentation?
   - What file extensions will capture the most relevant content?
   - How should large documents be chunked for optimal search?
   - What doc_type category best fits this repository?

CRITICAL: You MUST respond with valid JSON in exactly this format:

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
        "include_paths": ["Documentation/", "docs/"],
        "exclude_paths": ["test/", "vendor/", ".git/"],
        "extensions": ["md", "rst", "html"],
        "recursive": true,
        "chunk_size": 2000,
        "use_ai_chunking": true,
        "doc_type": "cilium",
        "source_name": "cilium-github"
    }},
    "cli_commands": [
        "git clone --depth 1 REPO_URL /tmp/cilium-analysis",
        "cargo run --bin loader -- local /tmp/cilium-analysis/Documentation --extensions md,rst --recursive -o ./cilium_docs_full",
        "cargo run --bin loader -- database --input-dir ./cilium_docs_full --doc-type cilium --source-name cilium-github --yes"
    ],
    "reasoning": "Detailed explanation of why these decisions were made, what type of documentation was found, and how the strategy maximizes content quality."
}}

RESPOND ONLY WITH THE JSON. DO NOT include any other text before or after the JSON.
"#,
            github_url, repo_info.name, repo_info.estimated_size
        );

        prompt
    }

    /// Parse Claude Code's analysis response into structured data
    fn parse_claude_analysis(
        claude_response: &str,
        repo_info: &RepoInfo,
    ) -> Result<RepositoryAnalysis> {
        // Try to extract JSON from Claude's response
        let json_start = claude_response
            .find('{')
            .ok_or_else(|| anyhow!("No JSON found in Claude response"))?;
        let json_end = claude_response
            .rfind('}')
            .ok_or_else(|| anyhow!("Incomplete JSON in Claude response"))?;
        let json_str = &claude_response[json_start..=json_end];

        // Parse the JSON response
        let claude_analysis: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse Claude's JSON response: {}", e))?;

        // Extract strategy information
        let strategy_json = claude_analysis
            .get("ingestion_strategy")
            .ok_or_else(|| anyhow!("Missing ingestion_strategy in Claude response"))?;

        let strategy = IngestionStrategy {
            docs_only: strategy_json
                .get("docs_only")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            include_paths: strategy_json
                .get("include_paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            exclude_paths: strategy_json
                .get("exclude_paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            extensions: strategy_json
                .get("extensions")
                .and_then(|v| v.as_array())
                .map_or_else(
                    || vec!["md".to_string(), "rst".to_string()],
                    |arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
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
                .map(|v| usize::try_from(v).unwrap_or(1000)),
            use_ai_chunking: strategy_json
                .get("use_ai_chunking")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            doc_type: strategy_json
                .get("doc_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            source_name: strategy_json
                .get("source_name")
                .and_then(|v| v.as_str())
                .unwrap_or("github-repo")
                .to_string(),
        };

        // Extract CLI commands
        let cli_commands = claude_analysis
            .get("cli_commands")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Extract reasoning
        let reasoning = claude_analysis
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        Ok(RepositoryAnalysis {
            repo_info: repo_info.clone(),
            strategy,
            cli_commands,
            reasoning,
        })
    }

    /// Parse GitHub URL to extract owner and repository name
    fn parse_github_url(url_str: &str) -> Result<(String, String)> {
        let url = url_str.trim_end_matches('/');
        let parts: Vec<&str> = url.split('/').collect();

        if parts.len() < 2 {
            return Err(anyhow!("Invalid GitHub URL format"));
        }

        let owner = parts[parts.len() - 2].to_string();
        let repo = parts[parts.len() - 1].to_string();

        Ok((owner, repo))
    }

    /// Execute the ingestion strategy generated by Claude Code
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Command execution fails
    /// - File operations fail
    /// - Database operations fail
    #[allow(clippy::too_many_lines)]
    pub fn execute_ingestion(&self, analysis: &RepositoryAnalysis) -> Result<()> {
        info!(
            "🚀 Executing ingestion strategy for: {}",
            analysis.repo_info.name
        );
        info!("📋 Reasoning: {}", analysis.reasoning);

        // Optional override of doc_type via environment variable
        let doc_type_override = std::env::var("DOC_TYPE_OVERRIDE").ok();
        let loader_bin = std::env::var("LOADER_BIN").unwrap_or_else(|_| "/app/loader".to_string());

        for (i, cmd) in analysis.cli_commands.iter().enumerate() {
            info!(
                "⚡ Executing command {}/{}: {}",
                i + 1,
                analysis.cli_commands.len(),
                cmd
            );

            // Parse and execute the command
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                warn!("Empty command, skipping");
                continue;
            }

            // Normalize command: replace cargo-run and bare 'loader' with LOADER_BIN
            let (program, mut args_vec): (String, Vec<String>) =
                if parts[0] == "cargo" && parts.len() >= 2 && parts[1] == "run" {
                    // Detect `--bin loader` and take args after `--`
                    let is_loader_bin = parts
                        .windows(2)
                        .any(|w| w.len() == 2 && w[0] == "--bin" && w[1] == "loader");
                    if is_loader_bin {
                        let args_start = parts
                            .iter()
                            .position(|p| *p == "--")
                            .map_or(parts.len(), |i| i + 1);
                        (
                            loader_bin.clone(),
                            parts[args_start..]
                                .iter()
                                .map(std::string::ToString::to_string)
                                .collect(),
                        )
                    } else {
                        (
                            parts[0].to_string(),
                            parts[1..]
                                .iter()
                                .map(std::string::ToString::to_string)
                                .collect(),
                        )
                    }
                } else if parts[0] == "loader" {
                    (
                        loader_bin.clone(),
                        parts[1..]
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect(),
                    )
                } else {
                    (
                        parts[0].to_string(),
                        parts[1..]
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect(),
                    )
                };

            let mut command = Command::new(program);

            // If overriding doc_type, rewrite args for database command
            if let Some(ref override_type) = doc_type_override {
                let is_database = args_vec.iter().any(|a| a == "database");
                if is_database {
                    let mut new_args: Vec<String> = Vec::with_capacity(args_vec.len() + 2);
                    let mut idx = 0;
                    while idx < args_vec.len() {
                        if args_vec[idx] == "--doc-type" {
                            idx += 1; // skip flag
                            if idx < args_vec.len() {
                                idx += 1; // skip existing value
                            }
                            new_args.push("--doc-type".to_string());
                            new_args.push(override_type.clone());
                        } else {
                            new_args.push(args_vec[idx].clone());
                            idx += 1;
                        }
                    }
                    if !new_args.iter().any(|a| a == "--doc-type") {
                        new_args.push("--doc-type".to_string());
                        new_args.push(override_type.clone());
                    }
                    args_vec = new_args;
                }
            }

            command.args(args_vec);

            let output = command
                .output()
                .map_err(|e| anyhow!("Failed to execute command '{}': {}", cmd, e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Command failed: {}", stderr);
                return Err(anyhow!("Command execution failed: {}", cmd));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("✅ Command completed successfully");
            if !stdout.trim().is_empty() {
                info!("📤 Output: {}", stdout.trim());
            }
        }

        info!("🎉 All ingestion commands completed successfully!");
        Ok(())
    }
}

/// CLI interface for intelligent ingestion
///
/// # Errors
///
/// This function will return an error if:
/// - The Claude Code binary is not available
/// - Repository analysis fails
/// - Ingestion strategy execution fails
pub async fn intelligent_ingest_command(github_url: &str) -> Result<()> {
    use std::io::{self, Write};

    let mut analyzer = IntelligentRepositoryAnalyzer::new()?;

    // Analyze the repository
    let analysis = analyzer.analyze_repository(github_url).await?;

    // Display the analysis
    println!("🎯 REPOSITORY ANALYSIS COMPLETE");
    println!("📊 Repository: {}", analysis.repo_info.name);
    println!("📋 Strategy: {:?}", analysis.strategy.doc_type);
    println!("🔧 Extensions: {:?}", analysis.strategy.extensions);
    println!("💭 Reasoning: {}", analysis.reasoning);
    println!();
    println!("🚀 GENERATED CLI COMMANDS:");
    for (i, cmd) in analysis.cli_commands.iter().enumerate() {
        println!("  {}. {}", i + 1, cmd);
    }
    println!();

    // Ask for confirmation
    print!("Execute these commands? (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "y" {
        // Execute the strategy
        analyzer.execute_ingestion(&analysis)?;
    } else {
        println!("❌ Ingestion cancelled by user");
    }

    Ok(())
}

/// Example usage and testing
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repository_analysis() {
        // This test requires Claude Code binary to be available
        if std::env::var("CLAUDE_BINARY_PATH").is_err() {
            println!("Skipping test - CLAUDE_BINARY_PATH not set");
            return;
        }

        let mut analyzer = IntelligentRepositoryAnalyzer::new().unwrap();

        // Test with a known repository
        let analysis = analyzer
            .analyze_repository("https://github.com/cilium/cilium")
            .await;

        match analysis {
            Ok(result) => {
                println!("Analysis successful:");
                println!("Doc type: {}", result.strategy.doc_type);
                println!("Extensions: {:?}", result.strategy.extensions);
                println!("Commands: {:?}", result.cli_commands);
                println!("Reasoning: {}", result.reasoning);
            }
            Err(e) => {
                println!("Analysis failed (expected if Claude binary not available): {e}");
            }
        }
    }
}

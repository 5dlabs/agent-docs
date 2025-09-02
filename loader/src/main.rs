//! AI-enabled Document Ingestion CLI
//!
//! This binary provides command-line access to the intelligent document ingestion system.
//! It supports ingesting documents from GitHub repositories, web pages, local files,
//! and provides an interactive mode for batch processing.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::fmt;

use loader::intelligent::{ClaudeIntelligentLoader, DocumentSource, IntelligentLoader};
use loader::loaders::RateLimiter;
use loader::parsers::UniversalParser;

/// AI-enabled Document Ingestion CLI
#[derive(Parser)]
#[command(name = "doc-ingest")]
#[command(about = "Intelligent document ingestion with AI-powered discovery")]
#[command(version = "0.1.0")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Maximum number of concurrent requests
    #[arg(long, default_value = "10")]
    max_concurrent: usize,

    /// Local repository path (alternative to cloning)
    #[arg(long)]
    local_repo: Option<std::path::PathBuf>,

    /// Chunk size for document processing
    #[arg(long, default_value = "2000")]
    chunk_size: usize,

    /// Chunk overlap for document processing
    #[arg(long, default_value = "200")]
    chunk_overlap: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest documents from a GitHub repository
    Github {
        /// GitHub repository URL (e.g., <https://github.com/user/repo>)
        url: String,

        /// Specific path within the repository (optional)
        #[arg(long)]
        path: Option<String>,

        /// Include documentation files only
        #[arg(long)]
        docs_only: bool,

        /// Output directory for processed documents
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },

    /// Ingest documents from a web page or documentation site
    Web {
        /// Web URL to ingest
        url: String,

        /// Maximum depth for crawling (0 = single page only)
        #[arg(long, default_value = "1")]
        max_depth: usize,

        /// Follow external links
        #[arg(long)]
        follow_external: bool,

        /// Output directory for processed documents
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },

    /// Ingest local files or directories
    Local {
        /// Path to local file or directory
        path: PathBuf,

        /// File extensions to include (comma-separated)
        #[arg(long, default_value = "md,rs,py,js,ts,json,yaml,yml,toml,txt")]
        extensions: String,

        /// Recursive directory traversal
        #[arg(long)]
        recursive: bool,

        /// Output directory for processed documents
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },

    /// Interactive batch processing mode
    Interactive {
        /// Configuration file path
        #[arg(long)]
        config: Option<PathBuf>,

        /// Output directory for processed documents
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    fmt().with_max_level(log_level).init();

    info!("🚀 Starting AI Document Ingestion CLI");

    // Initialize components
    let _rate_limiter = RateLimiter::new();
    let _parser = UniversalParser::new(cli.chunk_size, cli.chunk_overlap);
    let mut loader = match ClaudeIntelligentLoader::new() {
        Ok(loader) => loader,
        Err(e) => {
            eprintln!("Failed to initialize Claude loader: {}", e);
            eprintln!("Make sure ANTHROPIC_API_KEY environment variable is set");
            std::process::exit(1);
        }
    };

    // Execute the requested command
    match cli.command {
        Commands::Github {
            url,
            path,
            docs_only,
            output,
        } => {
            handle_github_command(
                &mut loader,
                &url,
                path.as_deref(),
                docs_only,
                output.as_path(),
            )
            .await?;
        }
        Commands::Web {
            url,
            max_depth,
            follow_external,
            output,
        } => {
            handle_web_command(
                &mut loader,
                &url,
                max_depth,
                follow_external,
                output.as_path(),
            )
            .await?;
        }
        Commands::Local {
            path,
            extensions,
            recursive,
            output,
        } => {
            handle_local_command(
                &mut loader,
                path.as_path(),
                &extensions,
                recursive,
                output.as_path(),
            )
            .await?;
        }
        Commands::Interactive { config, output } => {
            handle_interactive_command(&mut loader, config.as_deref(), output.as_path());
        }
    }

    info!("✅ Document ingestion completed successfully!");
    Ok(())
}

async fn handle_github_command(
    loader: &mut ClaudeIntelligentLoader,
    url: &str,
    path: Option<&str>,
    docs_only: bool,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("📚 Ingesting GitHub repository: {}", url);

    // Create document source
    let source = if let Some(p) = path {
        DocumentSource::GithubFile {
            url: url.to_string(),
            path: p.to_string(),
        }
    } else {
        DocumentSource::GithubRepo {
            url: url.to_string(),
            docs_only,
        }
    };

    // Discover and extract documents
    let documents = loader.extract_from_source(source).await?;

    info!("📄 Found {} documents", documents.len());

    // Process and save documents
    process_and_save_documents(documents, output).await?;

    Ok(())
}

async fn handle_web_command(
    loader: &mut ClaudeIntelligentLoader,
    url: &str,
    max_depth: usize,
    follow_external: bool,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🌐 Ingesting web content from: {}", url);

    let source = DocumentSource::WebPage {
        url: url.to_string(),
        max_depth,
        follow_external,
    };

    let documents = loader.extract_from_source(source).await?;
    info!("📄 Found {} web documents", documents.len());

    process_and_save_documents(documents, output).await?;

    Ok(())
}

async fn handle_local_command(
    loader: &mut ClaudeIntelligentLoader,
    path: &std::path::Path,
    extensions: &str,
    recursive: bool,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔍 Scanning local repository: {}", path.display());

    // Scan the local filesystem for documentation files
    let doc_files = scan_local_repository(path, extensions, recursive)?;
    info!("Found {} potential documentation files", doc_files.len());

    if doc_files.is_empty() {
        info!(
            "No documentation files found with extensions: {}",
            extensions
        );
        return Ok(());
    }

    // Use Claude to analyze and prioritize the documentation files
    let prioritized_files = analyze_local_files_with_claude(loader, &doc_files).await?;
    info!(
        "Claude prioritized {} files for processing",
        prioritized_files.len()
    );

    // Process the prioritized files
    process_prioritized_files(loader, &prioritized_files, output).await?;

    Ok(())
}

/// Scan local repository for documentation files
fn scan_local_repository(
    path: &std::path::Path,
    extensions: &str,
    recursive: bool,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let extensions: Vec<&str> = extensions.split(',').map(str::trim).collect();
    let mut doc_files = Vec::new();

    fn scan_dir(
        dir: &std::path::Path,
        extensions: &[&str],
        recursive: bool,
        files: &mut Vec<std::path::PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if recursive && !path.ends_with(".git") {
                    scan_dir(&path, extensions, recursive, files)?;
                }
            } else if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    if extensions.contains(&ext_str) {
                        files.push(path);
                    }
                }
            }
        }
        Ok(())
    }

    scan_dir(path, &extensions, recursive, &mut doc_files)?;
    Ok(doc_files)
}

/// Use Claude to analyze and prioritize local documentation files
async fn analyze_local_files_with_claude(
    loader: &mut ClaudeIntelligentLoader,
    doc_files: &[std::path::PathBuf],
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    if doc_files.is_empty() {
        return Ok(Vec::new());
    }

    // Build a summary of the repository structure for Claude
    let mut file_summary = String::new();
    for (i, file_path) in doc_files.iter().enumerate() {
        if let Some(file_name) = file_path.file_name() {
            if let Some(parent) = file_path.parent() {
                file_summary.push_str(&format!(
                    "{}. {} (in {})\n",
                    i + 1,
                    file_name.to_string_lossy(),
                    parent.display()
                ));
            }
        }
        if file_summary.len() > 10000 {
            // Limit summary size
            file_summary.push_str("... (truncated)\n");
            break;
        }
    }

    let analysis_prompt = format!(
        r#"Analyze this list of documentation files from a repository and prioritize the most important ones for ingestion:

Files found:
{}

Please prioritize files based on:
1. README files (highest priority)
2. Main documentation files (docs/, Documentation/, etc.)
3. API documentation
4. Configuration guides
5. Examples and tutorials

Return your analysis in JSON format with the following structure:
{{
    "prioritized_files": [
        {{
            "index": 1,
            "priority_score": 10,
            "reason": "Main README file"
        }},
        ...
    ]
}}

Only include the top 20-30 most important files, focusing on quality over quantity."#,
        file_summary
    );

    // Use Claude to analyze
    let analysis_response = loader.llm_client.summarize(&analysis_prompt).await?;
    info!("Claude analysis response: {}", analysis_response);

    // For now, return first 10 files as prioritized (we'll parse Claude's response later)
    let prioritized_count = std::cmp::min(10, doc_files.len());
    Ok(doc_files[..prioritized_count].to_vec())
}

/// Process the prioritized files
async fn process_prioritized_files(
    loader: &mut ClaudeIntelligentLoader,
    prioritized_files: &[std::path::PathBuf],
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "🎯 Processing {} prioritized files",
        prioritized_files.len()
    );

    let mut all_documents = Vec::new();

    for (i, file_path) in prioritized_files.iter().enumerate() {
        info!(
            "📄 Processing file {}/{}: {}",
            i + 1,
            prioritized_files.len(),
            file_path.display()
        );

        // Read the file content
        let content = tokio::fs::read_to_string(file_path).await?;
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();

        // Create document source
        let source = DocumentSource::LocalFile {
            path: file_path.clone(),
            extensions: vec!["md".to_string(), "rst".to_string(), "txt".to_string()],
            recursive: false,
        };

        // Extract documents from this source
        let documents = loader.extract_relevant(source).await?;
        all_documents.extend(documents);
    }

    info!("📊 Total documents extracted: {}", all_documents.len());

    // Process and save documents
    process_and_save_documents(all_documents, output).await?;

    Ok(())
}

fn handle_interactive_command(
    _loader: &mut ClaudeIntelligentLoader,
    _config: Option<&std::path::Path>,
    _output: &std::path::Path,
) {
    info!("🎯 Starting interactive mode");

    // TODO: Implement interactive mode
    println!("Interactive mode is not yet implemented.");
    println!("Use specific commands instead:");
    println!("  --help          Show help");
    println!("  github <url>    Ingest GitHub repository");
    println!("  web <url>       Ingest web page");
    println!("  local <path>    Ingest local files");
}

async fn process_and_save_documents(
    documents: Vec<loader::loaders::DocPage>,
    output_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    tokio::fs::create_dir_all(output_dir).await?;

    info!(
        "💾 Saving {} documents to {:?}",
        documents.len(),
        output_dir
    );

    for (i, doc) in documents.iter().enumerate() {
        let filename = format!("{:04}_{}.json", i + 1, sanitize_filename(&doc.module_path));
        let filepath = output_dir.join(filename);

        let json_content = serde_json::to_string_pretty(doc)?;
        tokio::fs::write(&filepath, json_content).await?;

        info!("  ✓ Saved: {}", filepath.display());
    }

    info!("📊 Processing complete:");
    info!("  📁 Documents saved: {}", documents.len());
    info!("  📂 Output directory: {:?}", output_dir);

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_whitespace() => '_',
            c => c,
        })
        .collect::<String>()
        .trim_end_matches('_')
        .to_lowercase()
}

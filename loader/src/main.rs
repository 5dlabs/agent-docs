//! AI-enabled Document Ingestion CLI
//!
//! This binary provides command-line access to the intelligent document ingestion system.
//! It supports ingesting documents from GitHub repositories, web pages, local files,
//! and provides an interactive mode for batch processing.

use clap::{Parser, Subcommand};
use std::fmt::Write;
use std::path::PathBuf;
use tracing::{info, warn, Level};
use tracing_subscriber::fmt;

use loader::intelligent::{ClaudeIntelligentLoader, DocumentSource, IntelligentLoader};
use loader::intelligent_ingestion::IntelligentRepositoryAnalyzer;
use loader::loaders::RateLimiter;
use loader::parsers::UniversalParser;

// Database dependencies
use db::models::Document;
use db::queries::DocumentQueries;
use db::DatabasePool;
use uuid::Uuid;

/// Helper function to scan a directory for files with specific extensions
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

    /// Load processed documents into the database
    Database {
        /// Directory containing JSON files to load
        #[arg(short, long)]
        input_dir: PathBuf,

        /// Document type for all files in the directory
        #[arg(long, default_value = "cilium")]
        doc_type: String,

        /// Source name for the documents
        #[arg(long, default_value = "cilium-repository")]
        source_name: String,

        /// Batch size for database insertions
        #[arg(long, default_value = "100")]
        batch_size: usize,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Intelligent repository analysis and ingestion using Claude Code
    Intelligent {
        /// GitHub repository URL to analyze and ingest
        url: String,

        /// Skip confirmation prompt for command execution
        #[arg(long)]
        yes: bool,
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
            eprintln!("Failed to initialize Claude loader: {e}");
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
        Commands::Database {
            input_dir,
            doc_type,
            source_name,
            batch_size,
            yes,
        } => {
            handle_database_command(
                input_dir.as_path(),
                &doc_type,
                &source_name,
                batch_size,
                yes,
            )
            .await?;
        }
        Commands::Intelligent { url, yes } => {
            handle_intelligent_command(&url, yes).await?;
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
                let _ = writeln!(
                    file_summary,
                    "{}. {} (in {})",
                    i + 1,
                    file_name.to_string_lossy(),
                    parent.display()
                );
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
{file_summary}

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

Only include the top 20-30 most important files, focusing on quality over quantity."#
    );

    // Use Claude to analyze
    let analysis_response = loader.llm_client.summarize(&analysis_prompt).await?;
    info!("Claude analysis response: {}", analysis_response);

    // For now, return ALL files as prioritized (we'll parse Claude's response later)
    // This ensures we capture the complete documentation set
    let prioritized_count = doc_files.len();
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
        let _content = tokio::fs::read_to_string(file_path).await?;
        let _file_name = file_path.file_name().unwrap_or_default().to_string_lossy();

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

async fn handle_database_command(
    input_dir: &std::path::Path,
    doc_type: &str,
    source_name: &str,
    batch_size: usize,
    skip_confirmation: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🗄️ Loading documents from database");
    info!("  📂 Input directory: {:?}", input_dir);
    info!("  📄 Document type: {}", doc_type);
    info!("  🏷️ Source name: {}", source_name);
    info!("  📦 Batch size: {}", batch_size);

    // Check if input directory exists
    if !input_dir.exists() {
        return Err(format!("Input directory does not exist: {}", input_dir.display()).into());
    }

    // Initialize database connection
    let pool = DatabasePool::from_env().await?;
    info!("✅ Connected to database");

    // Collect all JSON files from the directory
    let mut json_files = Vec::new();
    let mut entries = tokio::fs::read_dir(input_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            json_files.push(path);
        }
    }

    if json_files.is_empty() {
        return Err(format!("No JSON files found in {}", input_dir.display()).into());
    }

    info!("📄 Found {} JSON files to process", json_files.len());

    // Load and parse JSON files
    let mut documents = Vec::new();
    for file_path in &json_files {
        info!("📖 Loading: {}", file_path.display());

        let content = tokio::fs::read_to_string(file_path).await?;
        let parsed_doc: serde_json::Value = serde_json::from_str(&content)?;

        // Convert to Document struct
        let doc = create_document_from_json(&parsed_doc, doc_type, source_name);
        documents.push(doc);
    }

    info!("✅ Loaded {} documents from JSON files", documents.len());

    // Confirmation prompt unless skipped
    if !skip_confirmation {
        println!();
        println!("🔍 SUMMARY:");
        println!("  📄 Documents to insert: {}", documents.len());
        println!("  📁 Source directory: {}", input_dir.display());
        println!("  🏷️ Document type: {doc_type}");
        println!("  🏷️ Source name: {source_name}");
        println!();
        println!("⚠️ This will insert all documents into the database.");
        println!("Do you want to continue? (y/N): ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            info!("❌ Database insertion cancelled by user");
            return Ok(());
        }
    }

    // Insert documents in batches
    let mut inserted_count = 0;
    let mut failed_count = 0;

    for (i, batch) in documents.chunks(batch_size).enumerate() {
        info!(
            "📦 Processing batch {} of {} (size: {})",
            i + 1,
            documents.len().div_ceil(batch_size),
            batch.len()
        );

        match DocumentQueries::batch_insert_documents(pool.pool(), batch).await {
            Ok(inserted_docs) => {
                inserted_count += inserted_docs.len();
                info!("  ✅ Inserted {} documents in batch", inserted_docs.len());
            }
            Err(e) => {
                failed_count += batch.len();
                warn!("  ❌ Failed to insert batch: {}", e);
            }
        }
    }

    // Final summary
    println!();
    println!("📊 DATABASE INSERTION COMPLETE:");
    println!("  ✅ Documents inserted: {inserted_count}");
    if failed_count > 0 {
        println!("  ❌ Documents failed: {failed_count}");
    }
    println!("  📄 Total processed: {}", documents.len());
    println!("  🏷️ Source: {source_name}");

    if failed_count == 0 {
        info!("🎉 All documents successfully inserted into database!");
    } else {
        warn!("⚠️ Some documents failed to insert. Check logs for details.");
    }

    Ok(())
}

fn create_document_from_json(
    json_doc: &serde_json::Value,
    doc_type: &str,
    source_name: &str,
) -> Document {
    // Extract fields from JSON, with defaults for missing fields
    let id = Uuid::new_v4(); // Generate new ID for database
    let doc_type = doc_type.to_string();
    let source_name = source_name.to_string();

    // Try to extract path from various possible fields
    let doc_path = json_doc
        .get("module_path")
        .or_else(|| json_doc.get("path"))
        .or_else(|| json_doc.get("file_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Extract content
    let content = json_doc
        .get("content")
        .or_else(|| json_doc.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract metadata (use the entire JSON as metadata, or empty object)
    let metadata = if let Some(meta) = json_doc.get("metadata") {
        meta.clone()
    } else {
        // Create basic metadata from the document
        serde_json::json!({
            "original_doc_type": doc_type,
            "source": source_name,
            "imported_at": chrono::Utc::now().to_rfc3339()
        })
    };

    // Extract token count if available
    let token_count = json_doc
        .get("token_count")
        .and_then(serde_json::Value::as_i64)
        .and_then(|v| i32::try_from(v).ok());

    Document {
        id,
        doc_type,
        source_name,
        doc_path,
        content,
        metadata,
        embedding: None,
        token_count,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
    }
}

async fn handle_intelligent_command(
    github_url: &str,
    skip_confirmation: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    info!("🧠 Starting intelligent repository analysis");
    info!("🔗 Repository: {}", github_url);

    // Initialize the intelligent analyzer
    let mut analyzer = IntelligentRepositoryAnalyzer::new()
        .map_err(|e| format!("Failed to initialize Claude Code analyzer: {e}"))?;

    // Analyze the repository using Claude Code
    let analysis = analyzer
        .analyze_repository(github_url)
        .await
        .map_err(|e| format!("Repository analysis failed: {e}"))?;

    // Display the analysis results
    println!();
    println!("🎯 CLAUDE CODE ANALYSIS COMPLETE");
    println!("{}", "=".repeat(50));
    println!("📊 Repository: {}", analysis.repo_info.name);
    println!("📋 Doc Type: {}", analysis.strategy.doc_type);
    println!("🔧 Extensions: {:?}", analysis.strategy.extensions);
    println!("📁 Include Paths: {:?}", analysis.strategy.include_paths);
    if !analysis.strategy.exclude_paths.is_empty() {
        println!("🚫 Exclude Paths: {:?}", analysis.strategy.exclude_paths);
    }
    println!(
        "📦 Chunking: {}",
        if analysis.strategy.use_ai_chunking {
            "AI-powered"
        } else {
            "Basic"
        }
    );
    println!();
    println!("💭 CLAUDE'S REASONING:");
    println!("{}", analysis.reasoning);
    println!();
    println!("🚀 GENERATED CLI COMMANDS:");
    for (i, cmd) in analysis.cli_commands.iter().enumerate() {
        println!("  {}. {}", i + 1, cmd);
    }
    println!();

    // Execute commands if confirmed
    if skip_confirmation {
        info!("⚡ Auto-executing commands (--yes flag provided)");
        analyzer.execute_ingestion(&analysis)?;
    } else {
        print!("Execute these Claude-generated commands? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            info!("⚡ Executing Claude's ingestion strategy");
            analyzer.execute_ingestion(&analysis)?;
        } else {
            println!("❌ Ingestion cancelled by user");
            return Ok(());
        }
    }

    println!("🎉 Intelligent ingestion completed successfully!");
    Ok(())
}

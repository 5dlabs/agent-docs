//! AI-enabled Document Ingestion CLI
//!
//! This binary provides command-line access to the intelligent document ingestion system.
//! It supports ingesting documents from GitHub repositories, web pages, local files,
//! and provides an interactive mode for batch processing.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::fmt;

use loader::intelligent::{ClaudeIntelligentLoader, DocumentSource};
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
        /// GitHub repository URL (e.g., https://github.com/user/repo)
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
    let mut loader = ClaudeIntelligentLoader::new();

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
            handle_interactive_command(&mut loader, config.as_deref(), output.as_path()).await?;
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
    info!("💾 Processing local files from: {:?}", path);

    let extensions: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();

    let source = DocumentSource::LocalFile {
        path: path.to_path_buf(),
        extensions: extensions.iter().map(|s| s.to_string()).collect(),
        recursive,
    };

    let documents = loader.extract_from_source(source).await?;
    info!("📄 Found {} local documents", documents.len());

    process_and_save_documents(documents, output).await?;

    Ok(())
}

async fn handle_interactive_command(
    _loader: &mut ClaudeIntelligentLoader,
    _config: Option<&std::path::Path>,
    _output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🎯 Starting interactive mode");

    // TODO: Implement interactive mode
    println!("Interactive mode is not yet implemented.");
    println!("Use specific commands instead:");
    println!("  --help          Show help");
    println!("  github <url>    Ingest GitHub repository");
    println!("  web <url>       Ingest web page");
    println!("  local <path>    Ingest local files");

    Ok(())
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

//! AI-enabled Document Ingestion CLI
//!
//! This binary provides command-line access to the intelligent document ingestion system.
//! Supported flows:
//! - Analyzer-driven ingest runs in the server; loader provides the execution primitives used by plans
//! - "local" (directly parse files from a local path and emit JSON documents)
//! - "database" (load previously emitted JSON docs into the DB)

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, warn, Level};
use tracing_subscriber::fmt;

use loader::parsers::{DocumentFormat, UniversalParser};

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
                // Skip directory symlinks to avoid recursion loops
                if let Ok(meta) = std::fs::symlink_metadata(&path) {
                    if meta.file_type().is_symlink() {
                        continue;
                    }
                }
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
    /// Parse files via the CLI (used in analyzer-generated plans)
    Cli {
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
    // Intelligent ingest moved to server via discovery crate
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
    let _parser = UniversalParser::new(cli.chunk_size, cli.chunk_overlap);

    // Execute the requested command
    match cli.command {
        Commands::Cli {
            path,
            extensions,
            recursive,
            output,
        } => {
            handle_cli_command(path.as_path(), &extensions, recursive, output.as_path()).await?;
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
        } // Intelligent ingest now handled by server (discovery)
    }

    info!("✅ Document ingestion completed successfully!");
    Ok(())
}

// GitHub and Web commands removed (legacy path relied on deprecated intelligent module).

async fn handle_cli_command(
    path: &std::path::Path,
    extensions: &str,
    recursive: bool,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔍 Scanning local repository: {}", path.display());

    // Scan the local filesystem for documentation files
    let doc_files = scan_local_repository(path, extensions, recursive)?;
    info!("Found {} candidate files", doc_files.len());

    if doc_files.is_empty() {
        info!(
            "No documentation files found with extensions: {}",
            extensions
        );
        return Ok(());
    }

    // Process files directly (no LLM prioritization needed here)
    process_local_files(&doc_files, output).await?;

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
/// Process local files by parsing content and emitting `DocPage` JSON
async fn process_local_files(
    files: &[std::path::PathBuf],
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let parser = UniversalParser::default();

    let mut documents = Vec::new();
    for (i, file_path) in files.iter().enumerate() {
        info!(
            "📄 Processing file {}/{}: {}",
            i + 1,
            files.len(),
            file_path.display()
        );

        let content = tokio::fs::read_to_string(file_path).await?;
        let path_str = file_path.to_string_lossy();
        let parsed = parser.parse(&content, &path_str).await?;

        let item_type = match parsed.format {
            DocumentFormat::Markdown => "markdown",
            DocumentFormat::Html => "html",
            DocumentFormat::Json => "json_config",
            DocumentFormat::Yaml => "yaml_config",
            DocumentFormat::Toml => "toml_config",
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::ApiSpec => "api_spec",
            DocumentFormat::Code => "code",
            DocumentFormat::PlainText => "plain_text",
            DocumentFormat::Unknown => "unknown",
        };

        let doc_page = loader::loaders::DocPage {
            url: format!("file://{path_str}"),
            content: parsed.text_content,
            item_type: item_type.to_string(),
            module_path: path_str.to_string(),
            extracted_at: chrono::Utc::now(),
        };
        documents.push(doc_page);
    }

    process_and_save_documents(documents, output).await?;
    Ok(())
}

// Interactive mode removed for now; analyzer-driven or direct subcommands are preferred.

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

    // Collect all JSON files from the directory (recursively)
    let mut json_files = Vec::new();
    // Reuse scan_dir helper to recursively gather .json files
    scan_dir(input_dir, &["json"], true, &mut json_files)?;

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

    // Extract metadata (use the entire JSON as metadata, or create enhanced metadata)
    let metadata = if let Some(meta) = json_doc.get("metadata") {
        meta.clone()
    } else {
        // Create enhanced metadata by analyzing content
        create_enhanced_metadata(&doc_type, &source_name, &content, &doc_path)
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

/// Create enhanced metadata by analyzing document content and structure
fn create_enhanced_metadata(
    doc_type: &str,
    source_name: &str,
    content: &str,
    doc_path: &str,
) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();

    // Basic metadata
    metadata.insert(
        "original_doc_type".to_string(),
        serde_json::Value::String(doc_type.to_string()),
    );
    metadata.insert(
        "source".to_string(),
        serde_json::Value::String(source_name.to_string()),
    );
    metadata.insert(
        "imported_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    // Determine format from file path and content
    let format = determine_document_format(doc_path, content);
    metadata.insert("format".to_string(), serde_json::Value::String(format));

    // Analyze content for topic, category, and complexity based on doc_type
    match doc_type {
        "jupiter" => {
            let (topic, category, complexity) = analyze_jupiter_content(content, doc_path);
            metadata.insert("topic".to_string(), serde_json::Value::String(topic));
            metadata.insert("category".to_string(), serde_json::Value::String(category));
            metadata.insert(
                "complexity".to_string(),
                serde_json::Value::String(complexity),
            );
        }
        "solana" => {
            let (topic, category, complexity) = analyze_solana_content(content, doc_path);
            metadata.insert("topic".to_string(), serde_json::Value::String(topic));
            metadata.insert("category".to_string(), serde_json::Value::String(category));
            metadata.insert(
                "complexity".to_string(),
                serde_json::Value::String(complexity),
            );
        }
        "talos" => {
            let (topic, category, complexity) = analyze_talos_content(content, doc_path);
            metadata.insert("topic".to_string(), serde_json::Value::String(topic));
            metadata.insert("category".to_string(), serde_json::Value::String(category));
            metadata.insert(
                "complexity".to_string(),
                serde_json::Value::String(complexity),
            );
        }
        _ => {
            // Generic analysis for other document types
            let (topic, complexity) = analyze_generic_content(content, doc_path);
            metadata.insert("topic".to_string(), serde_json::Value::String(topic));
            metadata.insert(
                "complexity".to_string(),
                serde_json::Value::String(complexity),
            );
        }
    }

    serde_json::Value::Object(metadata)
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

/// Analyze Jupiter-specific content for metadata
fn analyze_jupiter_content(content: &str, doc_path: &str) -> (String, String, String) {
    let content_lower = content.to_lowercase();
    let path_lower = doc_path.to_lowercase();

    // Determine topic based on content analysis
    let topic = if content_lower.contains("api")
        || content_lower.contains("endpoint")
        || content_lower.contains("request")
        || content_lower.contains("response")
        || path_lower.contains("api")
    {
        "apis"
    } else if content_lower.contains("swap")
        || content_lower.contains("trade")
        || content_lower.contains("trading")
        || content_lower.contains("exchange")
    {
        "trading"
    } else if content_lower.contains("liquidity")
        || content_lower.contains("pool")
        || content_lower.contains("lp")
    {
        "liquidity"
    } else if content_lower.contains("integration")
        || content_lower.contains("sdk")
        || content_lower.contains("library")
        || content_lower.contains("client")
    {
        "integration"
    } else if content_lower.contains("development")
        || content_lower.contains("dev")
        || content_lower.contains("build")
        || content_lower.contains("setup")
    {
        "development"
    } else {
        "trading" // Default for Jupiter
    }
    .to_string();

    // Determine category based on content analysis
    let category = if content_lower.contains("swap")
        || content_lower.contains("quote")
        || path_lower.contains("swap")
    {
        "swap-api"
    } else if content_lower.contains("token") || content_lower.contains("asset") {
        "token-api"
    } else if content_lower.contains("price") || content_lower.contains("pricing") {
        "price-api"
    } else if content_lower.contains("dex") || content_lower.contains("exchange") {
        "dex-integration"
    } else if content_lower.contains("wallet") || content_lower.contains("connect") {
        "wallet-integration"
    } else {
        "swap-api" // Default for Jupiter
    }
    .to_string();

    // Determine complexity based on content characteristics
    let complexity = if content.len() < 1000
        || content_lower.contains("getting started")
        || content_lower.contains("quick start")
        || content_lower.contains("introduction")
        || path_lower.contains("intro")
        || path_lower.contains("basic")
    {
        "beginner"
    } else if content.len() > 5000
        || content_lower.contains("advanced")
        || content_lower.contains("complex")
        || content.matches("```").count() > 5
        || path_lower.contains("advanced")
    {
        "advanced"
    } else {
        "intermediate"
    }
    .to_string();

    (topic, category, complexity)
}

/// Analyze Solana-specific content for metadata
fn analyze_solana_content(content: &str, doc_path: &str) -> (String, String, String) {
    let content_lower = content.to_lowercase();
    let path_lower = doc_path.to_lowercase();

    let topic =
        if content_lower.contains("consensus") || content_lower.contains("proof of stake") {
            "consensus"
        } else if content_lower.contains("network") || content_lower.contains("rpc") {
            "networking"
        } else if content_lower.contains("validator") || content_lower.contains("staking") {
            "validators"
        } else if content_lower.contains("crypto")
            || content_lower.contains("hash")
            || content_lower.contains("signature")
        {
            "cryptography"
        } else {
            "development"
        }
        .to_string();

    let category = if path_lower.contains("diagram") || content_lower.contains("diagram") {
        "architecture-diagrams"
    } else if path_lower.contains("crypto") || content_lower.contains("zk") {
        "zk-cryptography"
    } else if path_lower.contains("sequence") {
        "sequence-diagrams"
    } else {
        "core"
    }
    .to_string();

    let complexity = determine_complexity_by_content(content, doc_path);

    (topic, category, complexity)
}

/// Analyze Talos-specific content for metadata
fn analyze_talos_content(content: &str, doc_path: &str) -> (String, String, String) {
    let content_lower = content.to_lowercase();

    let topic = if content_lower.contains("kubernetes") || content_lower.contains("k8s") {
        "kubernetes"
    } else if content_lower.contains("config") || content_lower.contains("configuration") {
        "configuration"
    } else if content_lower.contains("network") || content_lower.contains("networking") {
        "networking"
    } else if content_lower.contains("security") || content_lower.contains("secure") {
        "security"
    } else {
        "configuration"
    }
    .to_string();

    // Talos doesn't have predefined categories in tools.json, so we'll use a generic approach
    let category = "talos-os".to_string();

    let complexity = determine_complexity_by_content(content, doc_path);

    (topic, category, complexity)
}

/// Analyze generic content for basic metadata
fn analyze_generic_content(content: &str, doc_path: &str) -> (String, String) {
    let content_lower = content.to_lowercase();

    let topic = if content_lower.contains("api") {
        "apis"
    } else if content_lower.contains("config") {
        "configuration"
    } else if content_lower.contains("tutorial") || content_lower.contains("guide") {
        "development"
    } else {
        "general"
    }
    .to_string();

    let complexity = determine_complexity_by_content(content, doc_path);

    (topic, complexity)
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

// Intelligent command removed; discovery is handled by server

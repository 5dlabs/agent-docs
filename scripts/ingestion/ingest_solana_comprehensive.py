#!/usr/bin/env python3
"""
Comprehensive Solana Documentation Ingestion

Ingests ALL documentation from the Anza-xyz/agave repository including:
- Markdown files (.md, .mdx)
- ASCII art diagrams (.bob)  
- Message sequence charts (.msc)
- PDF technical specifications (metadata + basic text if possible)

This enhanced version captures the complete documentation ecosystem.
"""

import os
import sys
import asyncio
import asyncpg
from typing import List, Dict, Optional
from dataclasses import dataclass
from datetime import datetime
import openai
from dotenv import load_dotenv
from pgvector.asyncpg import register_vector
import re
import json
from pathlib import Path

# Add the project root to Python path for imports
sys.path.append(os.path.join(os.path.dirname(__file__), '../..'))

# Load environment variables
load_dotenv()

@dataclass
class SolanaDocument:
    """Represents a Solana documentation file"""
    file_path: str
    relative_path: str
    title: str
    content: str
    file_type: str
    category: str
    metadata: Dict

class ComprehensiveSolanaProcessor:
    """Processes all types of Solana documentation from the Agave repository"""
    
    def __init__(self, repo_path: str = "solana-agave-full"):
        self.repo_path = Path(repo_path)
        self.base_path = Path(".")
        
    def categorize_document(self, relative_path: str, file_type: str) -> str:
        """Enhanced categorization for all file types"""
        path_lower = relative_path.lower()
        
        # Special categories for new file types
        if file_type == "bob":
            return "architecture-diagrams"
        elif file_type == "msc":
            return "sequence-diagrams"
        elif file_type == "pdf":
            if "zk-docs" in path_lower:
                return "zk-cryptography"
            return "technical-specs"
        
        # Existing markdown categorization
        if "docs/src/consensus" in path_lower:
            return "consensus"
        elif "docs/src/cli" in path_lower:
            return "cli"
        elif "docs/src/validator" in path_lower:
            return "validator"
        elif "docs/src/runtime" in path_lower:
            return "runtime"
        elif "docs/src/proposals" in path_lower:
            return "proposals"
        elif "docs/src/operations" in path_lower:
            return "operations"
        elif "docs/src" in path_lower:
            return "core-docs"
        
        # Module-specific documentation
        elif "readme.md" in path_lower:
            parts = Path(relative_path).parts
            if len(parts) > 1:
                return f"module-{parts[-2]}"
            return "module-readme"
        
        # Top-level files
        elif relative_path.count("/") == 0:
            return "project-root"
        
        # Default categorization by directory
        else:
            first_dir = Path(relative_path).parts[0]
            return f"module-{first_dir}"
    
    def extract_title_from_content(self, content: str, file_path: str, file_type: str) -> str:
        """Enhanced title extraction for different file types"""
        
        if file_type in ["md", "mdx"]:
            # Check for frontmatter title first
            if content.startswith('---'):
                frontmatter_end = content.find('---', 3)
                if frontmatter_end != -1:
                    frontmatter = content[3:frontmatter_end]
                    title_match = re.search(r'^title:\s*(.+)$', frontmatter, re.MULTILINE)
                    if title_match:
                        return title_match.group(1).strip('"\'')
            
            # Look for H1 header (# Title)
            lines = content.strip().split('\n')
            for line in lines:
                line = line.strip()
                if line.startswith('# '):
                    return line[2:].strip()
            
            # Look for H2 header (## Title) 
            for line in lines:
                line = line.strip()
                if line.startswith('## '):
                    return line[3:].strip()
        
        elif file_type == "bob":
            # Extract title from BOB diagram comments or filename
            lines = content.strip().split('\n')
            # Look for title in comments
            for line in lines[:5]:  # Check first few lines
                line = line.strip()
                if line.startswith('#') or line.startswith('//'):
                    potential_title = line.lstrip('#/').strip()
                    if len(potential_title) > 3 and len(potential_title) < 100:
                        return potential_title
            
            # Use filename-based title for BOB files
            filename = Path(file_path).stem
            return f"{filename.replace('-', ' ').replace('_', ' ').title()} Diagram"
        
        elif file_type == "msc":
            # Message Sequence Chart
            lines = content.strip().split('\n')
            for line in lines[:10]:
                if line.strip().startswith('msc') and '{' in line:
                    return "Message Sequence Chart"
            filename = Path(file_path).stem
            return f"{filename.replace('-', ' ').replace('_', ' ').title()} Sequence"
        
        elif file_type == "pdf":
            # PDF files - use filename
            filename = Path(file_path).stem
            return f"{filename.replace('-', ' ').replace('_', ' ').title()} (PDF)"
        
        # Default fallback
        filename = Path(file_path).stem
        if filename.lower() == 'readme':
            parent = Path(file_path).parent.name
            return f"{parent.title()} Module"
        
        return filename.replace('-', ' ').replace('_', ' ').title()
    
    def clean_content(self, content: str, file_type: str) -> str:
        """Clean content based on file type"""
        if file_type in ["md", "mdx"]:
            # Remove excessive whitespace from markdown
            content = re.sub(r'\n\s*\n\s*\n', '\n\n', content)
            content = re.sub(r'^\s*\n', '', content)
            return content.strip()
        
        elif file_type in ["bob", "msc"]:
            # Preserve ASCII art formatting
            return content.strip()
        
        elif file_type == "pdf":
            # For PDFs, we'll just note it's a PDF - actual text extraction would need additional libraries
            return f"[PDF Document: {Path(content).name}]\n\nThis is a PDF technical specification. Content requires PDF reader to view."
        
        return content.strip()
    
    def discover_documentation_files(self) -> List[str]:
        """Find all documentation files of supported types"""
        print("🔍 Discovering ALL Solana documentation files...")
        
        doc_files = []
        supported_extensions = ['.md', '.mdx', '.bob', '.msc', '.pdf']
        
        # Search for all supported documentation files
        for ext in supported_extensions:
            pattern = f"*{ext}"
            for doc_file in self.repo_path.rglob(pattern):
                relative_path = str(doc_file.relative_to(self.repo_path))
                
                # Skip certain directories/files
                skip_patterns = [
                    'node_modules/',
                    '.git/',
                    'target/',
                    'build/',
                    'dist/',
                    # Skip files that are likely not documentation
                    'CODEOWNERS',
                    'NOTICE',
                ]
                
                if any(pattern in relative_path for pattern in skip_patterns):
                    continue
                    
                doc_files.append(str(doc_file))
        
        # Count by type
        type_counts = {}
        for file_path in doc_files:
            ext = Path(file_path).suffix[1:]  # Remove dot
            type_counts[ext] = type_counts.get(ext, 0) + 1
        
        print(f"  📚 Found {len(doc_files)} documentation files:")
        for file_type, count in sorted(type_counts.items()):
            print(f"    - {file_type}: {count} files")
        
        return doc_files
    
    def process_documentation_file(self, file_path: str) -> Optional[SolanaDocument]:
        """Process a single documentation file of any supported type"""
        try:
            relative_path = str(Path(file_path).relative_to(self.repo_path))
            file_extension = Path(file_path).suffix[1:].lower()  # Remove dot and lowercase
            
            # Read file content based on type
            if file_extension == "pdf":
                # For PDFs, we'll store metadata and reference
                with open(file_path, 'rb') as f:
                    file_size = len(f.read())
                content = file_path  # Store path for reference
            else:
                # Text-based files
                with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                    raw_content = f.read()
                
                if not raw_content.strip():
                    return None
                
                content = self.clean_content(raw_content, file_extension)
            
            # Extract title
            title = self.extract_title_from_content(content, file_path, file_extension)
            
            # Categorize
            category = self.categorize_document(relative_path, file_extension)
            
            # Enhanced metadata
            metadata = {
                'source_type': file_extension,
                'file_path': relative_path,
                'category': category,
                'file_size': len(content) if file_extension != "pdf" else file_size,
                'extracted_at': datetime.utcnow().isoformat(),
                'repository': 'anza-xyz/agave',
                'file_extension': file_extension
            }
            
            # Add specific metadata for PDFs
            if file_extension == "pdf":
                metadata['is_pdf'] = True
                metadata['pdf_path'] = relative_path
                # Create basic content description for PDFs
                content = f"""# {title}

**File Type:** PDF Technical Specification
**Location:** {relative_path}
**Repository:** anza-xyz/agave

This is a PDF document containing technical specifications for Solana's {title.lower()}. The PDF contains detailed mathematical proofs, algorithms, and implementation details.

**To access the full content:** This document requires a PDF reader. The file is located at `{relative_path}` in the Agave repository.

**Category:** Technical specification document
**Format:** Portable Document Format (PDF)
"""
            
            # Create document
            doc = SolanaDocument(
                file_path=file_path,
                relative_path=relative_path,
                title=title,
                content=content,
                file_type=file_extension,
                category=category,
                metadata=metadata
            )
            
            return doc
            
        except Exception as e:
            print(f"  ❌ Error processing {file_path}: {e}")
            return None
    
    def process_all_files(self) -> List[SolanaDocument]:
        """Process all documentation files"""
        print("📝 Processing ALL Solana documentation files...")
        
        doc_files = self.discover_documentation_files()
        documents = []
        
        for i, file_path in enumerate(doc_files):
            print(f"  Processing {i+1}/{len(doc_files)}: {Path(file_path).relative_to(self.repo_path)}")
            
            doc = self.process_documentation_file(file_path)
            if doc:
                documents.append(doc)
        
        print(f"  ✅ Processed {len(documents)} documents successfully")
        
        # Print comprehensive category summary
        categories = {}
        file_types = {}
        for doc in documents:
            categories[doc.category] = categories.get(doc.category, 0) + 1
            file_types[doc.file_type] = file_types.get(doc.file_type, 0) + 1
        
        print(f"  📊 Categories found:")
        for category, count in sorted(categories.items()):
            print(f"    - {category}: {count} documents")
        
        print(f"  📄 File types processed:")
        for file_type, count in sorted(file_types.items()):
            print(f"    - {file_type}: {count} documents")
        
        return documents

class EmbeddingGenerator:
    """Generates embeddings using OpenAI API"""
    
    def __init__(self, api_key: str):
        self.client = openai.Client(api_key=api_key)
    
    def generate_embedding(self, text: str, doc_title: str = "") -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Truncate text if too long (OpenAI embedding model limit is 8192 tokens)
        MAX_CHARS = 30_000  # ~7500 tokens (conservative estimate: 4 chars/token)
        if len(text) > MAX_CHARS:
            print(f"    ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        try:
            response = self.client.embeddings.create(
                model="text-embedding-3-large",
                input=text
            )
            return response.data[0].embedding
        except Exception as e:
            print(f"    ❌ Embedding generation failed for '{doc_title}': {e}")
            raise

class DatabaseManager:
    """Manages database operations"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_documents(self, documents: List[SolanaDocument], embeddings: List[List[float]]):
        """Store documents and embeddings in database"""
        print("💾 Storing in database...")
        
        conn = await asyncpg.connect(self.database_url)
        
        # Register vector type for pgvector
        await register_vector(conn)
        
        try:
            for i, (doc, embedding) in enumerate(zip(documents, embeddings)):
                print(f"  Storing {i+1}/{len(documents)}: {doc.title} ({doc.file_type})")
                
                # Use relative path as doc_path for uniqueness
                doc_path = doc.relative_path
                
                await conn.execute("""
                    INSERT INTO documents (
                        doc_type, source_name, doc_path, content, 
                        metadata, embedding, token_count
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (doc_type, source_name, doc_path) 
                    DO UPDATE SET 
                        content = EXCLUDED.content,
                        metadata = EXCLUDED.metadata,
                        embedding = EXCLUDED.embedding,
                        token_count = EXCLUDED.token_count,
                        updated_at = CURRENT_TIMESTAMP
                """, 
                    'solana',  # doc_type
                    'Solana Agave',  # source_name
                    doc_path,  # doc_path
                    doc.content,  # content
                    json.dumps(doc.metadata),  # metadata (convert dict to JSON string)
                    embedding,  # embedding (pgvector format)
                    len(doc.content) // 4  # estimated token count
                )
            
            print(f"  ✅ Stored {len(documents)} documents in database")
            
        finally:
            await conn.close()

async def main():
    """Main ingestion process"""
    print("🚀 Starting COMPREHENSIVE Solana Documentation Ingestion")
    print("📋 Supported formats: Markdown (.md, .mdx), ASCII Diagrams (.bob), Sequence Charts (.msc), PDFs (.pdf)")
    
    # Check environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Check if repository exists
    if not Path("solana-agave-full").exists():
        print("❌ solana-agave-full repository not found. Please clone it first:")
        print("   git clone https://github.com/anza-xyz/agave.git solana-agave-full")
        sys.exit(1)
    
    # Initialize components
    processor = ComprehensiveSolanaProcessor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Process all documentation
    documents = processor.process_all_files()
    
    if not documents:
        print("❌ No documents found to process")
        return
    
    print(f"📋 Processing {len(documents)} comprehensive documents...")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, doc in enumerate(documents):
        print(f"  Generating embedding {i+1}/{len(documents)} for: {doc.title} ({doc.file_type})")
        embedding = embedding_generator.generate_embedding(doc.content, doc.title)
        embeddings.append(embedding)
    
    # Store in database
    await db_manager.store_documents(documents, embeddings)
    
    print("🎉 COMPREHENSIVE Solana documentation ingestion completed successfully!")
    print(f"📊 Summary:")
    print(f"  - Total documents: {len(documents)}")
    print(f"  - Total embeddings: {len(embeddings)}")
    
    # Show detailed breakdown
    categories = {}
    file_types = {}
    for doc in documents:
        categories[doc.category] = categories.get(doc.category, 0) + 1
        file_types[doc.file_type] = file_types.get(doc.file_type, 0) + 1
    
    print(f"  - Categories:")
    for category, count in sorted(categories.items()):
        print(f"    * {category}: {count}")
    
    print(f"  - File Types:")
    for file_type, count in sorted(file_types.items()):
        print(f"    * {file_type}: {count}")

if __name__ == "__main__":
    asyncio.run(main())
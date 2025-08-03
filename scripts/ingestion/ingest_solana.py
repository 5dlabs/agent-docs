#!/usr/bin/env python3
"""
Solana Documentation Ingestion

Ingests all markdown documentation from the Anza-xyz/agave repository
into the Doc Server's harmonized database schema.
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
    category: str
    metadata: Dict

class SolanaDocProcessor:
    """Processes Solana documentation from the Agave repository"""
    
    def __init__(self, repo_path: str = "solana-agave"):
        self.repo_path = Path(repo_path)
        self.base_path = Path(".")
        
    def categorize_document(self, relative_path: str) -> str:
        """Categorize document based on its path"""
        path_lower = relative_path.lower()
        
        # Main documentation categories
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
            # Extract module name from path
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
    
    def extract_title_from_content(self, content: str, file_path: str) -> str:
        """Extract title from markdown content"""
        lines = content.strip().split('\n')
        
        # Look for H1 header (# Title)
        for line in lines:
            line = line.strip()
            if line.startswith('# '):
                return line[2:].strip()
        
        # Look for H2 header (## Title) 
        for line in lines:
            line = line.strip()
            if line.startswith('## '):
                return line[3:].strip()
        
        # Use filename if no header found
        filename = Path(file_path).stem
        if filename.lower() == 'readme':
            # Use parent directory name for README files
            parent = Path(file_path).parent.name
            return f"{parent.title()} Module"
        
        return filename.replace('-', ' ').replace('_', ' ').title()
    
    def clean_markdown_content(self, content: str) -> str:
        """Clean and format markdown content for better readability"""
        # Remove excessive whitespace
        content = re.sub(r'\n\s*\n\s*\n', '\n\n', content)
        
        # Fix common markdown issues
        content = re.sub(r'^\s*\n', '', content)  # Remove leading empty lines
        content = content.strip()
        
        return content
    
    def discover_markdown_files(self) -> List[str]:
        """Find all markdown files in the repository"""
        print("🔍 Discovering Solana documentation files...")
        
        md_files = []
        
        # Search for all .md files
        for md_file in self.repo_path.rglob("*.md"):
            # Skip certain directories/files
            relative_path = str(md_file.relative_to(self.repo_path))
            
            # Skip files we don't want
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
                
            md_files.append(str(md_file))
        
        print(f"  📚 Found {len(md_files)} markdown files")
        return md_files
    
    def process_markdown_file(self, file_path: str) -> Optional[SolanaDocument]:
        """Process a single markdown file"""
        try:
            relative_path = str(Path(file_path).relative_to(self.repo_path))
            
            # Read file content
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                raw_content = f.read()
            
            if not raw_content.strip():
                return None
            
            # Clean content
            content = self.clean_markdown_content(raw_content)
            
            # Extract title
            title = self.extract_title_from_content(content, file_path)
            
            # Categorize
            category = self.categorize_document(relative_path)
            
            # Create document
            doc = SolanaDocument(
                file_path=file_path,
                relative_path=relative_path,
                title=title,
                content=content,
                category=category,
                metadata={
                    'source_type': 'markdown',
                    'file_path': relative_path,
                    'category': category,
                    'file_size': len(content),
                    'extracted_at': datetime.utcnow().isoformat(),
                    'repository': 'anza-xyz/agave'
                }
            )
            
            return doc
            
        except Exception as e:
            print(f"  ❌ Error processing {file_path}: {e}")
            return None
    
    def process_all_files(self) -> List[SolanaDocument]:
        """Process all markdown files"""
        print("📝 Processing Solana documentation files...")
        
        md_files = self.discover_markdown_files()
        documents = []
        
        for i, file_path in enumerate(md_files):
            print(f"  Processing {i+1}/{len(md_files)}: {Path(file_path).relative_to(self.repo_path)}")
            
            doc = self.process_markdown_file(file_path)
            if doc:
                documents.append(doc)
        
        print(f"  ✅ Processed {len(documents)} documents successfully")
        
        # Print category summary
        categories = {}
        for doc in documents:
            categories[doc.category] = categories.get(doc.category, 0) + 1
        
        print(f"  📊 Categories found:")
        for category, count in sorted(categories.items()):
            print(f"    - {category}: {count} documents")
        
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
                print(f"  Storing {i+1}/{len(documents)}: {doc.title}")
                
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
    print("🚀 Starting Solana Documentation Ingestion")
    
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
    if not Path("solana-agave").exists():
        print("❌ solana-agave repository not found. Please clone it first:")
        print("   git clone https://github.com/anza-xyz/agave.git solana-agave")
        sys.exit(1)
    
    # Initialize components
    processor = SolanaDocProcessor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Process all documentation
    documents = processor.process_all_files()
    
    if not documents:
        print("❌ No documents found to process")
        return
    
    print(f"📋 Processing {len(documents)} documents...")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, doc in enumerate(documents):
        print(f"  Generating embedding {i+1}/{len(documents)} for: {doc.title}")
        embedding = embedding_generator.generate_embedding(doc.content, doc.title)
        embeddings.append(embedding)
    
    # Store in database
    await db_manager.store_documents(documents, embeddings)
    
    print("🎉 Solana documentation ingestion completed successfully!")
    print(f"📊 Summary:")
    print(f"  - Total documents: {len(documents)}")
    print(f"  - Total embeddings: {len(embeddings)}")
    
    # Show category breakdown
    categories = {}
    for doc in documents:
        categories[doc.category] = categories.get(doc.category, 0) + 1
    
    print(f"  - Categories:")
    for category, count in sorted(categories.items()):
        print(f"    * {category}: {count}")

if __name__ == "__main__":
    asyncio.run(main())
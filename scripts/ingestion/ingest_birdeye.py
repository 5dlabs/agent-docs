#!/usr/bin/env python3
"""
BirdEye API Documentation Ingestion Script

Extracts BirdEye API documentation from their embedded JSON and stores it
in the Doc Server harmonized database schema with embeddings.

This script automatically discovers ALL BirdEye API endpoints by scraping
their documentation navigation, then extracts the embedded JSON from each page.

Based on the WIP extract_birdeye_json.py approach, enhanced for comprehensive coverage.
"""

import urllib.parse
import html
import json
import requests
import time
import sys
import os
import asyncio
import asyncpg
from typing import List, Dict, Optional, Set
from dataclasses import dataclass
from datetime import datetime
import uuid
import re
from bs4 import BeautifulSoup

# Add the project root to Python path for imports
sys.path.append(os.path.join(os.path.dirname(__file__), '../..'))

@dataclass
class BirdEyeEndpoint:
    """Represents a BirdEye API endpoint"""
    url: str
    title: str
    method: str
    path: str
    content: str
    metadata: Dict

class BirdEyeExtractor:
    """Extracts BirdEye API documentation from their docs pages"""
    
    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Accept': 'application/json',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'identity',  # No compression to avoid Brotli issues
            'Connection': 'keep-alive',
        })
        self.base_url = "https://docs.birdeye.so"
    
    def discover_all_endpoints(self) -> List[str]:
        """Get all BirdEye API endpoints (using curated list since site is React-based)"""
        print("🔍 Getting BirdEye API endpoints...")
        
        # BirdEye uses React/JS rendering, so use curated comprehensive endpoint list
        print("  📋 Using comprehensive endpoint list (React site - JS rendered navigation)")
        return [
            # Core price endpoints
            "https://docs.birdeye.so/reference/get-defi-price",
            "https://docs.birdeye.so/reference/get-defi-multi_price", 
            "https://docs.birdeye.so/reference/post-defi-multi_price",
            "https://docs.birdeye.so/reference/get-defi-historical_price_unix",
            "https://docs.birdeye.so/reference/get-defi-history_price",
            "https://docs.birdeye.so/reference/get-defi-price_volume-single",
            "https://docs.birdeye.so/reference/post-defi-price_volume-multi",
            
            # Trading data endpoints  
            "https://docs.birdeye.so/reference/get-defi-txs-token",
            "https://docs.birdeye.so/reference/get-defi-txs-pair",
            "https://docs.birdeye.so/reference/get-defi-txs-token-seek_by_time",
            "https://docs.birdeye.so/reference/get-defi-txs-pair-seek_by_time",
            "https://docs.birdeye.so/reference/get-defi-v3-txs",
            "https://docs.birdeye.so/reference/get-defi-v3-token-txs", 
            "https://docs.birdeye.so/reference/get-defi-v3-txs-recent",
            
            # OHLCV endpoints
            "https://docs.birdeye.so/reference/get-defi-ohlcv",
            "https://docs.birdeye.so/reference/get-defi-ohlcv-pair",
            "https://docs.birdeye.so/reference/get-defi-ohlcv-base_quote",
            "https://docs.birdeye.so/reference/get-defi-v3-ohlcv",
            "https://docs.birdeye.so/reference/get-defi-v3-ohlcv-pair",
            
            # Token endpoints
            "https://docs.birdeye.so/reference/get-defi-token_overview",
            "https://docs.birdeye.so/reference/get-defi-token_security",
            "https://docs.birdeye.so/reference/get-defi-token_creation_info", 
            "https://docs.birdeye.so/reference/get-defi-tokenlist",
            "https://docs.birdeye.so/reference/get-defi-token_trending",
            "https://docs.birdeye.so/reference/get-defi-v3-token-list",
            "https://docs.birdeye.so/reference/get-defi-v3-token-list-scroll",
            "https://docs.birdeye.so/reference/get-defi-v3-token-meta-data-single",
            "https://docs.birdeye.so/reference/get-defi-v3-token-meta-data-multiple",
            "https://docs.birdeye.so/reference/get-defi-v3-token-market-data",
            "https://docs.birdeye.so/reference/get-defi-v3-token-market-data-multiple",
            "https://docs.birdeye.so/reference/get-defi-v3-token-trade-data-single",
            "https://docs.birdeye.so/reference/get-defi-v3-token-trade-data-multiple",
            
            # Wallet endpoints
            "https://docs.birdeye.so/reference/get-trader-gainers-losers",
            "https://docs.birdeye.so/reference/get-trader-txs-seek_by_time",
            "https://docs.birdeye.so/reference/get-v1-wallet-balance_change",
            "https://docs.birdeye.so/reference/get-v1-wallet-portfolio",
            "https://docs.birdeye.so/reference/get-v1-wallet-token_balance",
            "https://docs.birdeye.so/reference/get-v1-wallet-tx_list",
            "https://docs.birdeye.so/reference/get-v1-wallet-net_worth",
            
            # Utility endpoints
            "https://docs.birdeye.so/reference/get-defi-v3-search", 
            "https://docs.birdeye.so/reference/get-defi-networks",
            "https://docs.birdeye.so/reference/get-v1-wallet-list_supported_chain",
        ]
    
    def extract_endpoint_data(self, url: str) -> Optional[BirdEyeEndpoint]:
        """Extract API documentation from a BirdEye docs URL using dereference API"""
        try:
            # Convert regular docs URL to dereference API URL
            # From: https://docs.birdeye.so/reference/get-defi-price
            # To: https://docs.birdeye.so/birdeyedotso/api-next/v2/branches/1.0/reference/get-defi-price?dereference=true&reduce=false
            
            if '/reference/' not in url:
                print(f"  ❌ Invalid URL format: {url}")
                return None
                
            # Extract the endpoint slug
            slug = url.split('/reference/')[-1]
            api_url = f"https://docs.birdeye.so/birdeyedotso/api-next/v2/branches/1.0/reference/{slug}?dereference=true&reduce=false"
            
            print(f"  📡 Fetching: {api_url}")
            response = self.session.get(api_url, timeout=30)
            response.raise_for_status()
            
            # Parse JSON response directly
            data = response.json()
            
            # Extract clean API documentation from dereference response
            if 'data' not in data:
                print(f"  ❌ No data section in response")
                return None
                
            data_section = data['data']
            title = data_section.get('title', 'Unknown Endpoint')
            
            # Note: Removed overly aggressive rate limit detection
            # Those keywords appear in OpenAPI error response descriptions, not actual rate limiting
            
            # Extract method and path from API section
            method = "GET"  # Default
            path = "/unknown"
            api_spec = {}
            
            if 'api' in data_section:
                api_info = data_section['api']
                method = api_info.get('method', 'GET').upper()
                path = api_info.get('path', '/unknown')
                
                # Get the full OpenAPI schema if available
                if 'schema' in api_info:
                    api_spec = api_info['schema']
            
            # Build comprehensive content
            content_parts = []
            
            # Add title and basic info
            content_parts.append(f"# {title}")
            content_parts.append(f"**Method:** {method}")
            content_parts.append(f"**Path:** {path}")
            content_parts.append("")
            
            # Add content body/description from the dereference API
            content_body = ""
            if 'content' in data_section and 'body' in data_section['content']:
                content_body = data_section['content']['body']
                content_parts.append(f"**Description:**\n{content_body}")
                content_parts.append("")
            
            # Add OpenAPI specification if available
            if api_spec:
                content_parts.append("## OpenAPI Specification")
                content_parts.append("```json")
                content_parts.append(json.dumps(api_spec, indent=2))
                content_parts.append("```")
                content_parts.append("")
            
            # Add metadata information
            if 'metadata' in data_section:
                metadata_info = data_section['metadata']
                if metadata_info:
                    content_parts.append("## Metadata")
                    for key, value in metadata_info.items():
                        if value:
                            content_parts.append(f"**{key.title()}:** {value}")
                    content_parts.append("")
            
            full_content = "\n".join(content_parts)
            
            endpoint = BirdEyeEndpoint(
                url=url,
                title=title,
                method=method,
                path=path,
                content=full_content,
                metadata={
                    'source_url': url,
                    'api_url': api_url,
                    'api_method': method,
                    'api_path': path,
                    'title': title,
                    'openapi_spec': api_spec,
                    'content_body': content_body,
                    'extracted_at': datetime.utcnow().isoformat()
                }
            )
            
            print(f"  ✅ Extracted: {title} ({method} {path})")
            return endpoint
            
        except Exception as e:
            print(f"  ❌ Error processing {url}: {e}")
            return None

class EmbeddingGenerator:
    """Generates OpenAI embeddings for text content"""
    
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://api.openai.com/v1"
    
    async def generate_embedding(self, text: str) -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Truncate text if too long (OpenAI embedding model limit is 8192 tokens)
        MAX_CHARS = 30_000  # ~7500 tokens (conservative estimate: 4 chars/token)
        if len(text) > MAX_CHARS:
            print(f"  ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        payload = {
            "model": "text-embedding-3-large",
            "input": text,
            "encoding_format": "float"
        }
        
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{self.base_url}/embeddings",
                headers=headers,
                json=payload
            ) as response:
                if response.status != 200:
                    error_text = await response.text()
                    print(f"❌ OpenAI API Error {response.status}: {error_text}")
                    raise Exception(f"OpenAI API error: {response.status} - {error_text}")
                
                result = await response.json()
                return result['data'][0]['embedding']

class DatabaseManager:
    """Manages database operations for BirdEye documentation"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_endpoints(self, endpoints: List[BirdEyeEndpoint], embeddings: List[List[float]]):
        """Store BirdEye endpoints in the database"""
        conn = await asyncpg.connect(self.database_url)
        
        try:
            # First, create or update the document source
            await self.ensure_document_source(conn)
            
            # Store individual documents
            for endpoint, embedding in zip(endpoints, embeddings):
                await self.store_document(conn, endpoint, embedding)
            
            # Update source statistics
            await self.update_source_stats(conn)
            
        finally:
            await conn.close()
    
    async def ensure_document_source(self, conn):
        """Ensure BirdEye document source exists"""
        await conn.execute('''
            INSERT INTO document_sources (
                doc_type, source_name, version, config, enabled
            ) VALUES (
                'birdeye', 'birdeye-api', 'latest',
                $1, true
            ) ON CONFLICT (doc_type, source_name) DO UPDATE SET
                config = $1,
                updated_at = CURRENT_TIMESTAMP
        ''', json.dumps({
            'base_url': 'https://docs.birdeye.so',
            'extraction_method': 'data-initial-props scraping',
            'last_ingestion': datetime.utcnow().isoformat()
        }))
    
    async def store_document(self, conn, endpoint: BirdEyeEndpoint, embedding: List[float]):
        """Store a single BirdEye endpoint document"""
        doc_path = f"{endpoint.method.lower()}{endpoint.path.replace('/', '_')}"
        
        await conn.execute('''
            INSERT INTO documents (
                doc_type, source_name, doc_path, content, metadata, embedding, token_count
            ) VALUES (
                'birdeye', 'birdeye-api', $1, $2, $3, $4, $5
            ) ON CONFLICT (doc_type, source_name, doc_path) DO UPDATE SET
                content = $2,
                metadata = $3,
                embedding = $4,
                token_count = $5,
                updated_at = CURRENT_TIMESTAMP
        ''', doc_path, endpoint.content, json.dumps(endpoint.metadata), 
             embedding, len(endpoint.content.split()))
    
    async def update_source_stats(self, conn):
        """Update document source statistics"""
        await conn.execute('''
            UPDATE document_sources 
            SET 
                total_docs = (
                    SELECT COUNT(*) 
                    FROM documents 
                    WHERE doc_type = 'birdeye' AND source_name = 'birdeye-api'
                ),
                total_tokens = (
                    SELECT COALESCE(SUM(token_count), 0) 
                    FROM documents 
                    WHERE doc_type = 'birdeye' AND source_name = 'birdeye-api'
                )
            WHERE doc_type = 'birdeye' AND source_name = 'birdeye-api'
        ''')

async def main():
    """Main ingestion workflow"""
    print("🚀 Starting BirdEye API Documentation Ingestion")
    
    # Check for required environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Initialize components
    extractor = BirdEyeExtractor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Discover all BirdEye API endpoints automatically
    all_endpoints = extractor.discover_all_endpoints()
    
    # For testing: limit to first 3 endpoints (set to None for full run)
    test_mode = True  # Change to False for full ingestion
    if test_mode:
        endpoints_to_extract = all_endpoints[:3]
        print(f"🧪 TEST MODE: Processing only {len(endpoints_to_extract)} endpoints")
        for url in endpoints_to_extract:
            print(f"  - {url}")
    else:
        endpoints_to_extract = all_endpoints
    
    # Extract endpoints
    print(f"📋 Extracting {len(endpoints_to_extract)} BirdEye endpoints...")
    endpoints = []
    
    for i, url in enumerate(endpoints_to_extract):
        print(f"Processing {i+1}/{len(endpoints_to_extract)}: {url}")
        
        endpoint = extractor.extract_endpoint_data(url)
        if endpoint:
            endpoints.append(endpoint)
        
        # Rate limiting - wait between requests to be respectful
        if i < len(endpoints_to_extract) - 1:
            delay = 15 if len(endpoints_to_extract) > 20 else 10
            print(f"  💤 Waiting {delay} seconds...")
            time.sleep(delay)
    
    if not endpoints:
        print("❌ No endpoints successfully extracted")
        sys.exit(1)
    
    print(f"✅ Extracted {len(endpoints)} endpoints")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, endpoint in enumerate(endpoints):
        print(f"  Generating embedding {i+1}/{len(endpoints)} for: {endpoint.title}")
        embedding = await embedding_generator.generate_embedding(endpoint.content)
        embeddings.append(embedding)
        
        # Small delay to respect rate limits
        await asyncio.sleep(1)
    
    print(f"✅ Generated {len(embeddings)} embeddings")
    
    # Store in database
    print("💾 Storing in database...")
    await db_manager.store_endpoints(endpoints, embeddings)
    
    print("🎉 BirdEye ingestion completed successfully!")
    print(f"📊 Ingested {len(endpoints)} BirdEye API endpoints")

if __name__ == "__main__":
    # Import aiohttp here to avoid issues if not installed
    try:
        import aiohttp
    except ImportError:
        print("❌ aiohttp required. Install with: pip install aiohttp")
        sys.exit(1)
    
    try:
        import asyncpg
    except ImportError:
        print("❌ asyncpg required. Install with: pip install asyncpg")
        sys.exit(1)
    
    asyncio.run(main())
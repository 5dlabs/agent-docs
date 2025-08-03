#!/usr/bin/env python3
"""
BirdEye API Documentation Ingestion - Simple Approach

Downloads the complete OpenAPI spec in one request, then processes it locally.
Much more efficient than scraping individual pages.
"""

import json
import os
import sys
import asyncio
import asyncpg
import requests
from typing import List, Dict, Optional
from dataclasses import dataclass
from datetime import datetime
import openai
from dotenv import load_dotenv
from pgvector.asyncpg import register_vector

# Load environment variables
load_dotenv()

@dataclass
class BirdEyeEndpoint:
    """Represents a BirdEye API endpoint"""
    path: str
    method: str
    title: str
    description: str
    content: str
    metadata: Dict

class BirdEyeProcessor:
    """Processes BirdEye API documentation from OpenAPI spec"""
    
    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
            'Accept': 'application/json',
            'Accept-Encoding': 'identity',  # Avoid compression issues
        })
    
    def download_openapi_spec(self, output_file: str = "birdeye_openapi.json") -> Dict:
        """Download the complete BirdEye OpenAPI specification"""
        print("🌍 Downloading complete BirdEye OpenAPI specification...")
        
        # Use any endpoint to get the full spec - they all contain the same schema
        url = "https://docs.birdeye.so/birdeyedotso/api-next/v2/branches/1.0/reference/get-defi-price?dereference=true&reduce=false"
        
        print(f"  📡 Fetching: {url}")
        response = self.session.get(url, timeout=30)
        response.raise_for_status()
        
        data = response.json()
        
        # Extract the OpenAPI schema
        if 'data' not in data or 'api' not in data['data'] or 'schema' not in data['data']['api']:
            raise Exception("Could not find OpenAPI schema in response")
        
        openapi_spec = data['data']['api']['schema']
        
        # Save to disk
        with open(output_file, 'w') as f:
            json.dump(openapi_spec, f, indent=2)
        
        print(f"  ✅ Saved OpenAPI spec to {output_file}")
        print(f"  📊 Found {len(openapi_spec.get('paths', {}))} API paths")
        
        return openapi_spec
    
    def extract_endpoints_from_spec(self, openapi_spec: Dict) -> List[BirdEyeEndpoint]:
        """Extract individual endpoints from OpenAPI specification"""
        print("🔧 Processing OpenAPI specification...")
        
        endpoints = []
        paths = openapi_spec.get('paths', {})
        
        for path, path_data in paths.items():
            for method, endpoint_data in path_data.items():
                if method.upper() in ['GET', 'POST', 'PUT', 'DELETE', 'PATCH']:
                    
                    title = endpoint_data.get('summary', f'{method.upper()} {path}')
                    description = endpoint_data.get('description', '')
                    
                    # Build comprehensive content for this endpoint
                    content_parts = [
                        f"# {title}",
                        f"**Method:** {method.upper()}",
                        f"**Path:** {path}",
                        ""
                    ]
                    
                    if description:
                        content_parts.extend([f"**Description:** {description}", ""])
                    
                    # Add parameters
                    if 'parameters' in endpoint_data:
                        content_parts.extend(["## Parameters", ""])
                        for param in endpoint_data['parameters']:
                            param_name = param.get('name', 'unknown')
                            param_desc = param.get('description', 'No description')
                            param_required = param.get('required', False)
                            param_location = param.get('in', 'unknown')
                            
                            required_text = " (required)" if param_required else " (optional)"
                            content_parts.append(f"- **{param_name}** ({param_location}){required_text}: {param_desc}")
                        content_parts.append("")
                    
                    # Add response schemas
                    if 'responses' in endpoint_data:
                        content_parts.extend(["## Responses", ""])
                        for status_code, response_data in endpoint_data['responses'].items():
                            response_desc = response_data.get('description', 'No description')
                            content_parts.append(f"- **{status_code}**: {response_desc}")
                        content_parts.append("")
                    
                    # Add complete endpoint schema as JSON (but limit size)
                    content_parts.extend(["## Complete Schema", "```json"])
                    endpoint_json = json.dumps(endpoint_data, indent=2)
                    if len(endpoint_json) > 20_000:  # Limit to ~5K tokens
                        endpoint_json = endpoint_json[:20_000] + "... [TRUNCATED]"
                    content_parts.append(endpoint_json)
                    content_parts.extend(["```", ""])
                    
                    full_content = "\n".join(content_parts)
                    
                    endpoint = BirdEyeEndpoint(
                        path=path,
                        method=method.upper(),
                        title=title,
                        description=description,
                        content=full_content,
                        metadata={
                            'source': 'birdeye_openapi',
                            'extracted_at': datetime.utcnow().isoformat(),
                            'endpoint_data': endpoint_data
                        }
                    )
                    
                    endpoints.append(endpoint)
        
        print(f"  ✅ Extracted {len(endpoints)} endpoints")
        return endpoints

class EmbeddingGenerator:
    """Generates embeddings using OpenAI API"""
    
    def __init__(self, api_key: str):
        self.client = openai.Client(api_key=api_key)
    
    def generate_embedding(self, text: str) -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Ensure text fits in embedding model limits
        MAX_CHARS = 30_000  # ~7500 tokens
        if len(text) > MAX_CHARS:
            print(f"  ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        try:
            response = self.client.embeddings.create(
                model="text-embedding-3-large",
                input=text
            )
            return response.data[0].embedding
        except Exception as e:
            print(f"  ❌ Embedding generation failed: {e}")
            raise

class DatabaseManager:
    """Manages database operations"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_endpoints(self, endpoints: List[BirdEyeEndpoint], embeddings: List[List[float]]):
        """Store endpoints and embeddings in database"""
        print("💾 Storing in database...")
        
        conn = await asyncpg.connect(self.database_url)
        
        # Register vector type for pgvector
        await register_vector(conn)
        
        try:
            for i, (endpoint, embedding) in enumerate(zip(endpoints, embeddings)):
                print(f"  Storing {i+1}/{len(endpoints)}: {endpoint.title}")
                
                # Create doc_path as method + path
                doc_path = f"{endpoint.method} {endpoint.path}"
                
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
                    'birdeye',  # doc_type
                    'BirdEye API',  # source_name
                    doc_path,  # doc_path
                    endpoint.content,  # content
                    json.dumps(endpoint.metadata),  # metadata
                    embedding,  # embedding (pgvector format)
                    len(endpoint.content) // 4  # estimated token count
                )
            
            print(f"  ✅ Stored {len(endpoints)} endpoints in database")
            
        finally:
            await conn.close()

async def main():
    """Main ingestion process"""
    print("🚀 Starting BirdEye API Documentation Ingestion (Simple Approach)")
    
    # Check environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Initialize components
    processor = BirdEyeProcessor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Download OpenAPI spec (one request!)
    openapi_spec = processor.download_openapi_spec()
    
    # Extract individual endpoints
    endpoints = processor.extract_endpoints_from_spec(openapi_spec)
    
    print(f"📋 Processing {len(endpoints)} endpoints...")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, endpoint in enumerate(endpoints):
        print(f"  Generating embedding {i+1}/{len(endpoints)} for: {endpoint.title}")
        embedding = embedding_generator.generate_embedding(endpoint.content)
        embeddings.append(embedding)
    
    # Store in database
    await db_manager.store_endpoints(endpoints, embeddings)
    
    print("🎉 BirdEye ingestion completed successfully!")

if __name__ == "__main__":
    asyncio.run(main())
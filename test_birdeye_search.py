#!/usr/bin/env python3
"""
Test BirdEye semantic search for typical user queries
"""

import asyncio
import asyncpg
import openai
import os
from dotenv import load_dotenv
from pgvector.asyncpg import register_vector

load_dotenv()

async def test_semantic_search():
    """Test semantic search for BirdEye endpoints"""
    
    # Connect to database
    conn = await asyncpg.connect(os.getenv('DATABASE_URL'))
    
    # Register vector type for pgvector
    await register_vector(conn)
    
    # Initialize OpenAI client
    client = openai.Client(api_key=os.getenv('OPENAI_API_KEY'))
    
    # Test queries that match your use cases
    test_queries = [
        "Is there an endpoint that gives me wallet balance?",
        "How do I get the current price of a token?", 
        "What endpoints let me get trading data?",
        "Show me wallet portfolio information"
    ]
    
    for query in test_queries:
        print(f"\n🔍 Query: '{query}'")
        
        # Generate embedding for the query
        response = client.embeddings.create(
            model="text-embedding-3-large",
            input=query
        )
        query_embedding = response.data[0].embedding
        
        # Search for similar endpoints
        results = await conn.fetch("""
            SELECT 
                doc_path,
                substring(content, 1, 200) || '...' as preview,
                1 - (embedding <=> $1::vector) as similarity
            FROM documents 
            WHERE doc_type = 'birdeye'
            ORDER BY embedding <=> $1::vector
            LIMIT 3
        """, query_embedding)
        
        print("  📋 Top matches:")
        for i, result in enumerate(results, 1):
            print(f"    {i}. {result['doc_path']} (similarity: {result['similarity']:.3f})")
            print(f"       {result['preview'].replace(chr(10), ' ')}")
            print()
    
    await conn.close()

if __name__ == "__main__":
    asyncio.run(test_semantic_search())
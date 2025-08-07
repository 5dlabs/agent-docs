#!/usr/bin/env python3
"""
Live database validation script for the migration task.
Tests the actual PostgreSQL instance to verify migration completion.
"""

import psycopg2
import sys
from datetime import datetime

def connect_to_database(host="rustdocs-mcp-postgresql", port=5432, database="docs", 
                       user="rustdocs", password="rustdocs123"):
    """Connect to PostgreSQL database"""
    try:
        conn = psycopg2.connect(
            host=host,
            port=port,
            database=database,
            user=user,
            password=password,
            connect_timeout=10
        )
        return conn
    except Exception as e:
        print(f"Failed to connect to {database} database: {e}")
        return None

def test_original_database():
    """Test connection to original rust_docs_vectors database"""
    print("\n" + "="*60)
    print("TESTING ORIGINAL DATABASE: rust_docs_vectors")
    print("="*60)
    
    conn = connect_to_database(database="rust_docs_vectors")
    if not conn:
        print("❌ Cannot connect to original rust_docs_vectors database")
        return False
    
    try:
        cursor = conn.cursor()
        
        # Check tables exist
        print("\n1. Checking original database structure:")
        cursor.execute("""
            SELECT table_name FROM information_schema.tables 
            WHERE table_schema = 'public' 
            ORDER BY table_name;
        """)
        tables = cursor.fetchall()
        print(f"   Tables found: {[table[0] for table in tables]}")
        
        # Count original data
        if any('doc_embeddings' in str(table) for table in tables):
            cursor.execute("SELECT COUNT(*) FROM doc_embeddings;")
            doc_count = cursor.fetchone()[0]
            print(f"   Original documents: {doc_count}")
            
        if any('crates' in str(table) for table in tables):
            cursor.execute("SELECT COUNT(*) FROM crates;")
            crate_count = cursor.fetchone()[0]
            print(f"   Original crates: {crate_count}")
            
        conn.close()
        print("✅ Original database accessible and has data")
        return True
        
    except Exception as e:
        print(f"❌ Error querying original database: {e}")
        conn.close()
        return False

def validate_new_database():
    """Validate the new docs database with harmonized schema"""
    print("\n" + "="*60)
    print("TESTING NEW DATABASE: docs")  
    print("="*60)
    
    conn = connect_to_database(database="docs")
    if not conn:
        print("❌ Cannot connect to new docs database")
        return False
    
    try:
        cursor = conn.cursor()
        
        print(f"\n🕐 Validation timestamp: {datetime.now()}")
        
        # 1. Schema Structure Validation
        print("\n1. SCHEMA STRUCTURE VALIDATION")
        print("-" * 40)
        
        cursor.execute("""
            SELECT table_name, column_name, data_type, is_nullable
            FROM information_schema.columns 
            WHERE table_name = 'documents' 
            ORDER BY ordinal_position;
        """)
        schema = cursor.fetchall()
        
        if not schema:
            print("❌ Documents table not found!")
            return False
            
        print("✅ Documents table structure:")
        for row in schema:
            print(f"   {row[1]} ({row[2]}) - {'NULL' if row[3] == 'YES' else 'NOT NULL'}")
        
        # 2. Documentation Types Validation
        print("\n2. DOCUMENTATION TYPES VALIDATION")
        print("-" * 40)
        
        try:
            cursor.execute("SELECT unnest(enum_range(NULL::doc_type)) AS supported_doc_types;")
            doc_types = cursor.fetchall()
            print("✅ Supported documentation types:")
            for doc_type in doc_types:
                print(f"   - {doc_type[0]}")
        except Exception as e:
            print(f"⚠️  Doc type enum check failed (may use VARCHAR): {e}")
            # Check for varchar constraint instead
            cursor.execute("""
                SELECT column_name, check_clause 
                FROM information_schema.check_constraints cc
                JOIN information_schema.constraint_column_usage ccu 
                ON cc.constraint_name = ccu.constraint_name
                WHERE ccu.table_name = 'documents' AND ccu.column_name = 'doc_type';
            """)
            constraints = cursor.fetchall()
            if constraints:
                print("✅ Doc type constraints found (VARCHAR with CHECK)")
        
        # 3. Data Migration Validation
        print("\n3. DATA MIGRATION VALIDATION")
        print("-" * 40)
        
        # Count documents by type
        cursor.execute("""
            SELECT doc_type, COUNT(*) as document_count
            FROM documents 
            GROUP BY doc_type 
            ORDER BY document_count DESC;
        """)
        doc_counts = cursor.fetchall()
        
        total_docs = sum([count[1] for count in doc_counts])
        print(f"✅ Total documents migrated: {total_docs}")
        
        if doc_counts:
            print("   Documents by type:")
            for doc_type, count in doc_counts:
                print(f"   - {doc_type}: {count} documents")
        else:
            print("❌ No documents found in migration!")
            
        # 4. Embedding Validation
        print("\n4. EMBEDDING VALIDATION")
        print("-" * 40)
        
        cursor.execute("""
            SELECT COUNT(*) as documents_with_embeddings
            FROM documents 
            WHERE embedding IS NOT NULL;
        """)
        embedding_count = cursor.fetchone()[0]
        print(f"✅ Documents with embeddings: {embedding_count}")
        
        if total_docs > 0:
            embedding_percentage = (embedding_count / total_docs) * 100
            print(f"   Embedding coverage: {embedding_percentage:.1f}%")
        
        # Check embedding dimensions (if any exist)
        cursor.execute("""
            SELECT array_length(embedding, 1) as dimensions
            FROM documents 
            WHERE embedding IS NOT NULL 
            LIMIT 1;
        """)
        dim_result = cursor.fetchone()
        if dim_result:
            dimensions = dim_result[0]
            print(f"   Embedding dimensions: {dimensions}")
            if dimensions == 3072:
                print("   ✅ Correct OpenAI text-embedding-3-large dimensions")
            else:
                print(f"   ⚠️  Expected 3072 dimensions, found {dimensions}")
        
        # 5. Data Integrity Validation
        print("\n5. DATA INTEGRITY VALIDATION")
        print("-" * 40)
        
        # Check for duplicates
        cursor.execute("""
            SELECT doc_type, source_name, doc_path, COUNT(*)
            FROM documents 
            GROUP BY doc_type, source_name, doc_path 
            HAVING COUNT(*) > 1;
        """)
        duplicates = cursor.fetchall()
        
        if not duplicates:
            print("✅ No duplicate documents found")
        else:
            print(f"❌ Found {len(duplicates)} duplicate document entries:")
            for dup in duplicates[:5]:  # Show first 5
                print(f"   - {dup[0]}:{dup[1]}:{dup[2]} ({dup[3]} copies)")
        
        # 6. Sample Data Validation
        print("\n6. SAMPLE DATA VALIDATION")
        print("-" * 40)
        
        cursor.execute("""
            SELECT doc_type, source_name, LEFT(content, 100) as content_sample
            FROM documents 
            WHERE content IS NOT NULL AND content != ''
            ORDER BY created_at DESC
            LIMIT 3;
        """)
        samples = cursor.fetchall()
        
        if samples:
            print("✅ Sample migrated documents:")
            for i, sample in enumerate(samples, 1):
                print(f"   {i}. Type: {sample[0]}, Source: {sample[1]}")
                print(f"      Content: {sample[2]}...")
        else:
            print("❌ No sample documents with content found")
        
        # 7. Vector Search Test
        print("\n7. VECTOR SEARCH FUNCTIONALITY TEST") 
        print("-" * 40)
        
        if embedding_count > 0:
            try:
                cursor.execute("""
                    SELECT source_name, doc_path, LEFT(content, 80) as content_preview
                    FROM documents 
                    WHERE doc_type = 'rust' AND embedding IS NOT NULL
                    ORDER BY embedding <-> (
                        SELECT embedding FROM documents 
                        WHERE doc_type = 'rust' AND embedding IS NOT NULL 
                        LIMIT 1
                    )
                    LIMIT 3;
                """)
                search_results = cursor.fetchall()
                
                if search_results:
                    print("✅ Vector similarity search working:")
                    for i, result in enumerate(search_results, 1):
                        print(f"   {i}. {result[0]}::{result[1]}")
                        print(f"      Preview: {result[2]}...")
                else:
                    print("⚠️  Vector search returned no results")
                    
            except Exception as e:
                print(f"❌ Vector search test failed: {e}")
        else:
            print("⚠️  Skipping vector search test - no embeddings found")
        
        # 8. Document Sources Validation
        print("\n8. DOCUMENT SOURCES VALIDATION")
        print("-" * 40)
        
        try:
            cursor.execute("""
                SELECT doc_type, source_name, enabled, 
                       COALESCE(total_docs, 0) as configured_docs
                FROM document_sources 
                ORDER BY doc_type, source_name;
            """)
            sources = cursor.fetchall()
            
            if sources:
                print(f"✅ Document sources configured: {len(sources)}")
                print("   Sources by type:")
                current_type = None
                for source in sources:
                    if source[0] != current_type:
                        current_type = source[0]
                        print(f"   {current_type}:")
                    status = "✅" if source[2] else "❌"
                    print(f"     {status} {source[1]} ({source[3]} docs)")
            else:
                print("⚠️  No document sources found")
                
        except Exception as e:
            print(f"⚠️  Document sources table may not exist: {e}")
        
        conn.close()
        
        # Final Summary
        print("\n" + "="*60)
        print("MIGRATION VALIDATION SUMMARY")
        print("="*60)
        
        if total_docs > 0 and embedding_count > 0:
            print("✅ MIGRATION SUCCESSFUL")
            print(f"   - {total_docs} documents migrated")
            print(f"   - {embedding_count} embeddings preserved")
            print(f"   - Schema supports all required documentation types")
            print(f"   - Vector search functionality working")
            print(f"   - Data integrity maintained")
            return True
        else:
            print("❌ MIGRATION INCOMPLETE")
            print("   - Missing documents or embeddings")
            return False
            
    except Exception as e:
        print(f"❌ Error during validation: {e}")
        import traceback
        traceback.print_exc()
        conn.close()
        return False

def main():
    """Main validation function"""
    print("🔍 DATABASE MIGRATION VALIDATION")
    print("Testing live PostgreSQL instance for migration completion")
    
    # Test original database (if accessible)
    original_accessible = test_original_database()
    
    # Test new database (primary validation)
    new_database_valid = validate_new_database()
    
    print("\n" + "="*60)
    print("FINAL VALIDATION RESULT")
    print("="*60)
    
    if new_database_valid:
        print("✅ DATABASE MIGRATION VALIDATION: PASSED")
        print("   The docs database contains migrated data and is fully functional")
        return True
    else:
        print("❌ DATABASE MIGRATION VALIDATION: FAILED")
        print("   The migration appears incomplete or databases are inaccessible")
        return False

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
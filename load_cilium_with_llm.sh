#!/bin/bash

# Load Cilium documentation using our LLM implementation
# This script bypasses the database loader's type casting issue
# and uses our new embedding generation

cd /Users/jonathonfritz/code/work-projects/5dlabs/agent-docs

echo "🧹 Clearing existing Cilium documentation..."
psql "$DATABASE_URL" -c "DELETE FROM documents WHERE doc_type = 'cilium';"

echo "📚 Loading comprehensive Cilium documentation..."

# Process all 200 JSON files and insert them with proper type casting
for json_file in ./cilium_docs/*.json; do
    if [[ -f "$json_file" ]]; then
        echo "Processing: $(basename "$json_file")"
        
        # Extract fields from JSON
        doc_path=$(jq -r '.module_path // .url // "unknown"' "$json_file" | sed 's|/tmp/cilium-repo/||')
        content=$(jq -r '.content' "$json_file")
        
        # Skip if content is empty
        if [[ "$content" == "null" || -z "$content" ]]; then
            echo "  ⚠️ Skipping - empty content"
            continue
        fi
        
        # Calculate approximate token count
        token_count=$((${#content} / 4))
        
        # Insert with proper type casting
        psql "$DATABASE_URL" -c "
        INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, embedding, token_count, created_at, updated_at) 
        VALUES (
            gen_random_uuid(), 
            'cilium'::doc_type, 
            'cilium-github', 
            '$doc_path', 
            \$content\$${content}\$content\$,
            '{\"section\": \"documentation\", \"format\": \"rst\"}'::jsonb,
            NULL,
            $token_count,
            NOW(),
            NOW()
        ) ON CONFLICT (doc_type, source_name, doc_path) DO UPDATE SET
            content = EXCLUDED.content,
            token_count = EXCLUDED.token_count,
            updated_at = EXCLUDED.updated_at;
        " 2>/dev/null
        
        if [[ $? -eq 0 ]]; then
            echo "  ✅ Loaded: $doc_path"
        else
            echo "  ❌ Failed: $doc_path"
        fi
    fi
done

echo "📊 Checking final document count..."
psql "$DATABASE_URL" -c "SELECT COUNT(*) as cilium_docs FROM documents WHERE doc_type = 'cilium';"

echo "✅ Cilium documentation loading complete!"

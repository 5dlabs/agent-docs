# Adding New Tools - Step-by-Step Guide

## Overview

Adding a new tool to the agent-docs system requires updates in multiple layers due to the hybrid approach of dynamic configuration (JSON) and static database schema (enum-based).

## Current Architecture

- **Configuration Layer**: Dynamic (reads from `tools.json`)
- **Database Layer**: Static (enum-based with `DocType`)
- **Validation Layer**: Dynamic (extracts from configuration)

## Process for Adding a New Tool

### Step 1: Add Tool Configuration

**File**: `tools.json`

Add your new tool to the tools array:

```json
{
  "name": "new_protocol_query",
  "docType": "new_protocol",
  "title": "New Protocol Documentation Query",
  "description": "Search and retrieve information from New Protocol documentation",
  "enabled": true,
  "metadataHints": {
    "supported_formats": ["markdown", "json"],
    "supported_complexity_levels": ["beginner", "intermediate", "advanced"],
    "supported_categories": ["protocol", "documentation"],
    "supported_topics": ["api", "integration", "development"],
    "supports_api_version": false
  }
}
```

### Step 2: Add Database Enum Variant

**File**: `db/src/models.rs`

Add your new doc type to the `DocType` enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "doc_type", rename_all = "snake_case")]
pub enum DocType {
    Rust,
    Jupiter,
    Birdeye,
    Cilium,
    // ... existing variants
    NewProtocol,  // ← Add your new variant here
}
```

### Step 3: Add String Mapping

**File**: `db/src/models.rs` (in the `Display` implementation)

Add the string mapping for your new variant:

```rust
impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DocType::Rust => "rust",
            DocType::Jupiter => "jupiter",
            DocType::Birdeye => "birdeye",
            // ... existing mappings
            DocType::NewProtocol => "new_protocol",  // ← Add your mapping here
        };
        write!(f, "{s}")
    }
}
```

### Step 4: Add Migration Parser Support

**File**: `loader/src/bin/migrate.rs`

Add support in the migration parser:

```rust
fn parse_source_path(s: &str) -> Result<(DocType, PathBuf), String> {
    let (type_str, path_str) = s
        .split_once('=')
        .ok_or_else(|| "Source path must be in format 'type=path'".to_string())?;

    let doc_type = match type_str.to_lowercase().as_str() {
        "rust" => DocType::Rust,
        "jupiter" => DocType::Jupiter,
        "birdeye" => DocType::Birdeye,
        // ... existing cases
        "new_protocol" => DocType::NewProtocol,  // ← Add your case here
        _ => return Err(format!("Unknown document type: {type_str}")),
    };

    Ok((doc_type, PathBuf::from(path_str)))
}
```

### Step 5: Update SQL Migration

**File**: `mcp/src/bin/http_server.rs`

Update the SQL migration to include your new doc type:

```rust
let enum_sql = r"
    DO $$ BEGIN
        CREATE TYPE doc_type AS ENUM (
            'rust', 'jupiter', 'birdeye', 'cilium', 'talos',
            'meteora', 'raydium', 'solana', 'ebpf', 'rust_best_practices', 'new_protocol'  // ← Add your type here
        );
    EXCEPTION
        WHEN duplicate_object THEN null;
    END $$;
";
```

### Step 6: Update Test Database Setup

**File**: `scripts/setup_test_db.sql`

Add your new type to the test database enum:

```sql
CREATE TYPE doc_type AS ENUM (
    'rust',
    'jupiter',
    'birdeye',
    'cilium',
    'talos',
    'meteora',
    'raydium',
    'solana',
    'ebpf',
    'rust_best_practices',
    'new_protocol'  -- ← Add your type here
);
```

And add test data if needed:

```sql
INSERT INTO document_sources (doc_type, source_name, config, enabled)
VALUES ('new_protocol', 'test_new_protocol', '{"api_version": "v1"}', true);
```

### Step 7: Test Your Changes

Run the full test suite:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Step 8: Deploy and Run Migration

If this is a production change:

1. Deploy the updated code
2. Run the database migration to update the enum
3. Verify the new tool appears in the system

## Verification Checklist

- [ ] Tool appears in MCP server tool list
- [ ] Tool can be invoked successfully
- [ ] Database queries work with new doc type
- [ ] Migration scripts handle new type correctly
- [ ] All tests pass
- [ ] Clippy passes with no warnings

## Future Improvements

This process could be simplified by:

1. **Removing the enum entirely** and using strings everywhere (loses type safety)
2. **Creating an automated tool addition script** that makes all these changes
3. **Implementing runtime enum extension** for better dynamic behavior

## Example: Adding "Ethereum" Tool

Following the steps above for adding an "ethereum" tool:

1. **tools.json**: Add ethereum_query tool
2. **models.rs**: Add `DocType::Ethereum`
3. **models.rs**: Add `"ethereum"` mapping
4. **migrate.rs**: Add `"ethereum" => DocType::Ethereum`
5. **http_server.rs**: Add `'ethereum'` to SQL enum
6. **setup_test_db.sql**: Add `'ethereum'` to test enum

This ensures the new tool is fully integrated across all layers of the system.

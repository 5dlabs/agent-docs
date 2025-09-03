# Making the System Truly Dynamic

## Current Problem

The system has **3 layers** with different dynamism levels:

1. **✅ Configuration Layer**: Fully dynamic (reads from `tools.json`)
2. **❌ Database Layer**: Static (enum-based SQL schema)
3. **❌ Application Layer**: Partially dynamic (some hardcoded mappings)

## Option 1: String-Based Approach (Simplest)

**Remove the enum entirely** and use strings everywhere:

```rust
// Instead of:
pub enum DocType {
    Rust,
    Jupiter,
    Birdeye,
}

// Use:
pub type DocType = String;
```

**Pros:**
- ✅ Add any tool by just updating `tools.json`
- ✅ No code changes needed for new tools
- ✅ Simple and maintainable

**Cons:**
- ❌ Lose compile-time type safety
- ❌ Runtime errors instead of compile errors
- ❌ No IDE autocompletion for doc types

## Option 2: Configuration-Driven Enums

**Generate enum and mappings from JSON at build time:**

```rust
// build.rs - Generate this at compile time from tools.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocType {
    Rust,
    Jupiter,
    Birdeye,
    // ... generated from tools.json
}
```

**Pros:**
- ✅ Type safety maintained
- ✅ IDE autocompletion
- ✅ Compile-time error checking

**Cons:**
- ❌ Requires build-time code generation
- ❌ More complex build process
- ❌ Still need to rebuild for new tools

## Option 3: Runtime Enum Extension

**Load enum variants dynamically at runtime:**

```rust
pub struct DynamicEnum {
    variants: HashMap<String, usize>,
    reverse: Vec<String>,
}

impl DynamicEnum {
    pub fn from_config(config: &ToolsConfig) -> Self {
        // Build enum from tools.json
    }
}
```

**Pros:**
- ✅ Fully dynamic
- ✅ No rebuilds needed
- ✅ Type safety with runtime checks

**Cons:**
- ❌ Complex implementation
- ❌ Performance overhead
- ❌ Error-prone

## Option 4: Hybrid with Registry Pattern

**Keep static core types, make extensions dynamic:**

```rust
pub enum CoreDocType {
    Rust,      // Always available
    Jupiter,   // Always available
}

pub struct DocTypeRegistry {
    core_types: HashMap<String, CoreDocType>,
    extensions: HashMap<String, String>, // Dynamic extensions
}
```

## Recommendation

**For your use case, I'd recommend Option 1 (String-Based)** because:

1. **You want true dynamism** - add tools without code changes
2. **The trade-off is acceptable** - runtime validation can catch most errors
3. **Simple to implement** - just change `DocType` from enum to `String`

## Implementation Steps (Option 1)

1. **Change DocType to String** in `db/src/models.rs`
2. **Update all usages** to work with strings
3. **Remove hardcoded mappings** from Display impl
4. **Update SQL schema** to use TEXT instead of enum
5. **Update migrations** to handle dynamic types

After this, you could add any new tool by **just updating `tools.json`** - no code changes needed!

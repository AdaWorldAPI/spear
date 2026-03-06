# Spear - Claude Code Instructions

## Project Overview

Spear is a **columnar mail server** built on Lance format. Not a KV adapter - proper columnar storage where IMAP operations map to typed column access.

## Quick Start

```bash
cargo build
cargo test
```

## Architecture Invariants (CRITICAL)

These are **NON-NEGOTIABLE**. Violating any triggers immediate code review:

1. **No SQL strings** - Use DataFusion Expr API only
   ```rust
   // WRONG
   let filter = format!("uid = {}", uid);
   
   // RIGHT
   col("uid").eq(lit(uid))
   ```

2. **No opaque blobs** - Every Lance column has semantic type
   ```rust
   // WRONG
   Field::new("data", DataType::Binary, false)
   
   // RIGHT
   Field::new("from_addr", DataType::Utf8, true)
   ```

3. **No JSON** - Arrow/Lance native formats only

4. **Column pruning** - Never SELECT * without explicit need
   ```rust
   // WRONG
   table.query().execute()
   
   // RIGHT
   table.query().select(Select::Columns(vec!["uid", "flags"])).execute()
   ```

5. **Predicate pushdown** - Filters go to Lance, not post-filter

## Multi-Agent System

This project uses a multi-agent development pattern. Load the orchestration prompt:

```bash
cat .claude/ORCHESTRATION_PROMPT.md
```

### Agents

- **Orchestrator** - Coordinates team, maintains blackboard
- **Archaeologist** - Schema design, Lance encoding
- **QueryMaster** - DataFusion expressions
- **StorageOracle** - Lance/Arrow operations
- **ProtocolSage** - IMAP/JMAP semantics
- **Verifier** - Testing
- **Scribe** - Documentation

### Blackboard

State persists in `.claude/blackboard/state.json`. Update after significant changes.

## Key Files

| File | Purpose |
|------|---------|
| `src/schema/messages.rs` | Message table (24 columns) |
| `src/query/messages.rs` | DataFusion query builders |
| `src/content/mod.rs` | SHA256 content store |
| `docs/ARCHITECTURE.md` | Full architecture spec |
| `docs/INTEGRATION_PLAN.md` | Development roadmap |

## Current Phase

**Phase 1: Core Storage**

- [ ] Verify LanceDB 0.23 compilation
- [ ] Implement write operations
- [ ] Integration tests
- [ ] Verify columnar compression

## Dependencies

```toml
arrow = "56"
lancedb = "0.23"
datafusion = "49"
tokio = "1"
sha2 = "0.10"
```

## Testing

```bash
# All tests
cargo test

# Specific test
cargo test test_message_roundtrip

# With output
cargo test -- --nocapture
```

## Compression (Why This Matters)

Traditional mail servers: `message_id → serialized_blob`
- FETCH FLAGS: Deserialize entire 10KB blob, extract flags

Spear: Typed columns
- FETCH FLAGS: Read `flags` column only (3 bits/row, Dict encoded)
- For 10K messages: 4KB read vs 100MB

## DataFusion Expression Patterns

```rust
// Equality
col("uid").eq(lit(5))

// Range
col("uid").gt_eq(lit(1)).and(col("uid").lt_eq(lit(100)))

// LIKE
col("from_addr").like(lit("%alice%"))

// Array contains
array_has(col("flags"), lit("\\Seen"))

// Array NOT contains
array_has(col("flags"), lit("\\Seen")).not()
```

## Common Tasks

### Add a new column to messages

1. Edit `src/schema/messages.rs`
2. Add Field with proper DataType
3. Document encoding rationale
4. Update column index constants
5. Run tests

### Add a new query type

1. Edit `src/query/messages.rs`
2. Build Expr using col(), lit(), and(), etc.
3. Specify columns to select (column pruning!)
4. Add test in same file

### Store message body

1. Hash content with SHA256
2. Store via `content.put(data)`
3. Store hash in `body_ref` column
4. Retrieve via `content.get(hash)`

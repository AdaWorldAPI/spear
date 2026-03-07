# Spear → Stalwart Surgical Integration

## Mission

Add Spear (Lance columnar) as a storage backend to Stalwart mail server.
**Surgical**: Minimal changes, easy rebase on upstream, feature-gated.

---

## Phase 0: Reconnaissance (DO NOT WRITE CODE YET)

### 0.1 Clone and Analyze Stalwart

```bash
cd /tmp
git clone --depth 1 https://github.com/stalwartlabs/stalwart.git stalwart-analysis
cd stalwart-analysis
```

### 0.2 Find Store Traits

```bash
# Find the trait definitions
find crates/store/src -name "*.rs" | xargs grep -l "pub trait"
find crates/store/src -name "*.rs" | xargs grep -l "enum Store"

# Document what you find:
cat crates/store/src/lib.rs | head -200
```

**CHECKPOINT 0.2**: Before proceeding, list:
- [ ] Path to Store trait definition
- [ ] Path to BlobStore trait definition  
- [ ] Path to SearchStore trait definition
- [ ] The enum that wraps all backend implementations
- [ ] How existing backends (rocksdb, postgres) are structured

### 0.3 Analyze Existing Backend Structure

```bash
# Pick one backend to understand the pattern
ls -la crates/store/src/backend/
cat crates/store/src/backend/rocksdb/mod.rs | head -100

# Check Cargo.toml for feature flags
cat crates/store/Cargo.toml | grep -A 50 "\[features\]"
```

**CHECKPOINT 0.3**: Document:
- [ ] File structure of a backend (rocksdb or postgres)
- [ ] How it's feature-gated in Cargo.toml
- [ ] How the backend registers itself in the Store enum
- [ ] Required trait methods (list them with signatures)

### 0.4 Analyze Data Store Interface

```bash
# Find the actual trait methods we need to implement
grep -A 5 "async fn get" crates/store/src/*.rs
grep -A 5 "async fn set" crates/store/src/*.rs  
grep -A 5 "async fn delete" crates/store/src/*.rs
grep -A 5 "async fn iterate" crates/store/src/*.rs
```

**CHECKPOINT 0.4**: Create method inventory:
```
Store trait methods:
- [ ] get(key) -> ?
- [ ] set(key, value) -> ?
- [ ] delete(key) -> ?
- [ ] iterate(prefix) -> ?
- [ ] (list all others...)

BlobStore trait methods:
- [ ] put_blob(data) -> hash?
- [ ] get_blob(hash) -> data?
- [ ] (list all others...)

SearchStore trait methods:
- [ ] index(doc) -> ?
- [ ] search(query) -> ?
- [ ] (list all others...)
```

### 0.5 Determine Spear Mapping Strategy

For each Stalwart operation, plan how Spear handles it:

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Stalwart Operation     │ Spear Implementation           │ Columnar Benefit │
├────────────────────────────────────────────────────────────────────────────┤
│ get(key)               │ ?                              │ ?                │
│ set(key, value)        │ ?                              │ ?                │
│ iterate(prefix)        │ ?                              │ ?                │
│ put_blob(data)         │ content.put(data) → SHA256     │ Dedup            │
│ get_blob(hash)         │ content.get(hash)              │ -                │
│ fts_index(doc)         │ Lance FTS index                │ No Elasticsearch │
│ fts_search(query)      │ table.search(query)            │ Native           │
└────────────────────────────────────────────────────────────────────────────┘
```

**CHECKPOINT 0.5**: Answer these questions:
- [ ] Does Stalwart's KV interface have typed keys or opaque bytes?
- [ ] Can we preserve columnar benefits or must we fall back to KV?
- [ ] What's the minimal set of traits we MUST implement?
- [ ] What's optional/can be stubbed initially?

---

## Phase 1: Design Document (STILL NO CODE)

### 1.1 Write Integration Design

Based on Phase 0 findings, create:

```markdown
# Spear Backend Design

## Traits to Implement
1. [Trait name] - [X methods]
2. ...

## File Structure
crates/store/src/backend/spear/
├── mod.rs          # SpearStore struct + trait impls
├── data.rs         # KV operations mapped to Lance
├── blob.rs         # Content-addressed storage
├── search.rs       # FTS via Lance
└── convert.rs      # Stalwart types ↔ Arrow types

## Cargo.toml Changes
- Add "spear" feature
- Dependencies: spear, lancedb, arrow, datafusion, tokio

## lib.rs Changes  
- Add SpearStore variant to Store enum
- Feature-gate with #[cfg(feature = "spear")]

## Configuration
[store."spear"]
type = "spear"
path = "/var/data/spear"

## Mapping Strategy
[Document each KV operation → Lance operation]
```

**CHECKPOINT 1.1**: Design review
- [ ] Design doc complete
- [ ] All required traits identified
- [ ] Mapping strategy documented
- [ ] No ambiguity remaining

---

## Phase 2: Setup Fork

### 2.1 Create Clean Fork

```bash
cd /home/user  # or your workspace
git clone https://github.com/stalwartlabs/stalwart.git stalwart-spear
cd stalwart-spear
git remote add upstream https://github.com/stalwartlabs/stalwart.git
git checkout -b feature/spear-backend
```

### 2.2 Verify Clean Build First

```bash
# Must pass before we change anything
cargo build --release 2>&1 | tail -20
cargo test --workspace 2>&1 | tail -20
```

**CHECKPOINT 2.2**: 
- [ ] Stalwart builds clean
- [ ] Tests pass
- [ ] Note the baseline (X tests, Y warnings)

---

## Phase 3: Surgical Implementation

### 3.1 Create Backend Directory

```bash
mkdir -p crates/store/src/backend/spear
```

### 3.2 Add Feature Flag

Edit `crates/store/Cargo.toml`:

```toml
[features]
# ... existing features ...
spear = ["dep:spear", "dep:lancedb", "dep:arrow", "dep:datafusion"]

[dependencies]
# ... existing deps ...
spear = { git = "https://github.com/AdaWorldAPI/spear.git", optional = true }
lancedb = { version = "0.26", optional = true }
arrow = { version = "57", optional = true }
datafusion = { version = "52", features = ["nested_expressions"], optional = true }
```

**CHECKPOINT 3.2**:
```bash
# Verify no syntax errors
cargo check -p store 2>&1 | head -20
```

### 3.3 Create mod.rs Skeleton

`crates/store/src/backend/spear/mod.rs`:

```rust
//! Spear (Lance columnar) storage backend for Stalwart
//!
//! Provides columnar storage with BtrBlocks-style compression,
//! O(1) point lookups, and native FTS/vector search.

#[cfg(feature = "spear")]
mod data;
#[cfg(feature = "spear")]
mod blob;
#[cfg(feature = "spear")]
mod search;

#[cfg(feature = "spear")]
pub use self::data::*;
#[cfg(feature = "spear")]
pub use self::blob::*;
#[cfg(feature = "spear")]
pub use self::search::*;

use std::path::PathBuf;
use std::sync::Arc;

/// Spear storage backend
#[cfg(feature = "spear")]
pub struct SpearStore {
    db: Arc<spear::Spear>,
    path: PathBuf,
}

#[cfg(feature = "spear")]
impl SpearStore {
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, spear::Error> {
        let path = path.into();
        let db = spear::Spear::open(&path).await?;
        Ok(Self { 
            db: Arc::new(db),
            path,
        })
    }
}
```

**CHECKPOINT 3.3**:
```bash
cargo check -p store --features spear 2>&1 | head -30
# Should compile (with unused warnings OK for now)
```

### 3.4 Implement Data Store Trait

Create `crates/store/src/backend/spear/data.rs`:

```rust
//! KV operations mapped to Lance columnar storage
//!
//! Strategy:
//! - Stalwart uses opaque key-value pairs
//! - We store in Lance for future columnar benefits
//! - Phase 1: Simple KV table (key Binary, value Binary)
//! - Phase 2: Parse keys, create typed columns
```

[Implement based on Phase 0 findings - the actual trait signatures]

**CHECKPOINT 3.4**:
```bash
cargo check -p store --features spear 2>&1
# List remaining unimplemented trait methods
```

### 3.5 Implement Blob Store Trait

Create `crates/store/src/backend/spear/blob.rs`:

```rust
//! Blob storage via Spear content store
//!
//! Maps to SHA256-addressed content store
//! (Stalwart uses BLAKE3, we use SHA256 - may need adapter)
```

**CHECKPOINT 3.5**:
```bash
cargo check -p store --features spear 2>&1
```

### 3.6 Implement Search Store Trait

Create `crates/store/src/backend/spear/search.rs`:

```rust
//! Full-text search via Lance FTS
//!
//! Benefits:
//! - No external ElasticSearch
//! - Native FTS at ~900 queries/sec
//! - Bonus: Vector search for "similar messages"
```

**CHECKPOINT 3.6**:
```bash
cargo check -p store --features spear 2>&1
```

### 3.7 Register in Store Enum

Find and edit the Store enum (location from Phase 0):

```rust
pub enum Store {
    // ... existing variants ...
    
    #[cfg(feature = "spear")]
    Spear(Arc<SpearStore>),
}
```

**CHECKPOINT 3.7**:
```bash
# Full build with spear feature
cargo build -p store --features spear 2>&1
```

---

## Phase 4: Integration Tests

### 4.1 Basic Roundtrip

```rust
#[cfg(feature = "spear")]
#[tokio::test]
async fn test_spear_store_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let store = SpearStore::open(dir.path()).await.unwrap();
    
    // Test KV
    store.set(b"key1", b"value1").await.unwrap();
    let val = store.get(b"key1").await.unwrap();
    assert_eq!(val, Some(b"value1".to_vec()));
    
    // Test blob
    let hash = store.put_blob(b"hello world").await.unwrap();
    let data = store.get_blob(&hash).await.unwrap();
    assert_eq!(data, Some(b"hello world".to_vec()));
}
```

**CHECKPOINT 4.1**:
```bash
cargo test -p store --features spear test_spear 2>&1
```

### 4.2 Full Stalwart Test Suite

```bash
# Run full test suite with spear enabled
cargo test --workspace --features spear 2>&1 | tee test_results.txt

# Compare to baseline from Phase 2.2
# Should have same pass rate
```

**CHECKPOINT 4.2**:
- [ ] All existing tests still pass
- [ ] No regressions
- [ ] Spear-specific tests pass

---

## Phase 5: Verification Checklist

### 5.1 Surgical Verification

```bash
# Count changed files (should be minimal)
git diff --stat main..feature/spear-backend

# Verify changes are isolated
git diff main..feature/spear-backend -- crates/store/src/lib.rs
# Should only show: enum variant addition, feature-gated

git diff main..feature/spear-backend -- crates/store/Cargo.toml  
# Should only show: feature flag, optional deps
```

**CHECKPOINT 5.1**:
- [ ] < 10 files changed outside backend/spear/
- [ ] All changes are feature-gated
- [ ] No changes to existing backend code

### 5.2 Rebase Test

```bash
# Fetch latest upstream
git fetch upstream main

# Try rebase (should be clean)
git rebase upstream/main

# If conflicts, document them
```

**CHECKPOINT 5.2**:
- [ ] Rebase clean OR
- [ ] Conflicts documented and resolvable

### 5.3 Feature Isolation Test

```bash
# Build WITHOUT spear (existing behavior unchanged)
cargo build --workspace 2>&1 | tail -10

# Build WITH spear
cargo build --workspace --features spear 2>&1 | tail -10
```

**CHECKPOINT 5.3**:
- [ ] Builds without spear feature
- [ ] Builds with spear feature
- [ ] No cross-contamination

---

## Phase 6: Documentation

### 6.1 Update Stalwart Docs

Create `docs/storage/spear.md`:

```markdown
# Spear Storage Backend

Spear provides columnar storage using Lance format.

## Benefits
- BtrBlocks-style compression
- O(1) point lookups
- Native FTS (no ElasticSearch)
- Vector search for semantic queries

## Configuration
[store."spear"]
type = "spear"
path = "/var/data/stalwart/spear"

## When to Use
- High query volume (100s/sec)
- Need FTS without external services
- Want columnar compression benefits
```

### 6.2 Update README

Add to supported backends list.

---

## Decision Gates

### After Phase 0:
PROCEED if: All trait signatures documented, mapping strategy clear
STOP if: Stalwart architecture incompatible with columnar approach

### After Phase 1:
PROCEED if: Design doc approved, no ambiguity
STOP if: Design reveals fundamental issues

### After Phase 3:
PROCEED if: Compiles with feature flag
STOP if: Trait requirements incompatible with Spear

### After Phase 4:
PROCEED if: Tests pass, no regressions
STOP if: Tests fail, need architectural changes

### After Phase 5:
SHIP if: All checkpoints pass
ITERATE if: Minor issues to fix

---

## Invariants (Automatic STOP if Violated)

1. **No changes to existing backends** - Only add, never modify
2. **Feature-gated everything** - `#[cfg(feature = "spear")]` on all new code
3. **Tests must pass** - Both existing and new
4. **Clean rebase** - Must not conflict with upstream
5. **No JSON** - Spear doesn't use JSON anywhere

---

## Success Criteria

```
[ ] Phase 0 complete - Stalwart architecture understood
[ ] Phase 1 complete - Design doc written
[ ] Phase 2 complete - Fork created, baseline verified
[ ] Phase 3 complete - SpearStore compiles
[ ] Phase 4 complete - Tests pass
[ ] Phase 5 complete - Surgical verification passed
[ ] Phase 6 complete - Documentation updated

Ready to PR to AdaWorldAPI/stalwart
```

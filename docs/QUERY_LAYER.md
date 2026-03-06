# Spear Query Layer Specification

## Design Philosophy

**DataFusion Expr, not SQL strings.**

```rust
// WRONG
let filter = format!("mailbox_id = '{}' AND uid = {}", id, uid);

// RIGHT
col("mailbox_id").eq(lit(id)).and(col("uid").eq(lit(uid)))
```

Why:
- Type-safe (compiler catches mistakes)
- Composable (build complex queries programmatically)
- Optimizable (DataFusion pushes predicates to Lance)
- No escaping (no SQL injection, no quoting hell)

---

## IMAP → DataFusion Mapping

### SELECT

```
IMAP: SELECT INBOX

Expr: col("account_id").eq(lit(account_id))
        .and(col("path").eq(lit("INBOX")))

Table: folders
Columns: all
```

### FETCH

```
IMAP: FETCH 5 (FLAGS)

Expr: col("mailbox_id").eq(lit(mailbox_id))
        .and(col("uid").eq(lit(5)))

Table: messages
Columns: ["uid", "flags"]  ← ONLY these columns read
```

```
IMAP: FETCH 1:100 (ENVELOPE)

Expr: col("mailbox_id").eq(lit(mailbox_id))
        .and(col("uid").gt_eq(lit(1)))
        .and(col("uid").lt_eq(lit(100)))

Table: messages
Columns: ["uid", "date", "from_addr", "from_name", "to_addrs", 
          "cc_addrs", "bcc_addrs", "reply_to", "subject",
          "message_id", "in_reply_to"]
```

```
IMAP: FETCH 5 BODY[]

Expr: col("mailbox_id").eq(lit(mailbox_id))
        .and(col("uid").eq(lit(5)))

Table: messages
Columns: ["body_ref"]

Then: content_store.get(body_ref)
```

### SEARCH

```
IMAP: SEARCH FROM "alice"

Expr: col("mailbox_id").eq(lit(mailbox_id))
        .and(col("from_addr").like(lit("%alice%")))

Table: messages
Columns: ["uid"]
```

```
IMAP: SEARCH UNSEEN

Expr: col("mailbox_id").eq(lit(mailbox_id))
        .and(array_has(col("flags"), lit("\\Seen")).not())

Table: messages
Columns: ["uid"]
```

```
IMAP: SEARCH FROM "bob" UNSEEN SINCE 1-Jan-2025 SMALLER 10000

Expr: col("mailbox_id").eq(lit(mailbox_id))
        .and(col("from_addr").like(lit("%bob%")))
        .and(array_has(col("flags"), lit("\\Seen")).not())
        .and(col("internal_date").gt_eq(lit(timestamp)))
        .and(col("size").lt(lit(10000)))

Table: messages
Columns: ["uid"]
```

### STORE

```
IMAP: STORE 5 +FLAGS (\Seen)

Operation: UPDATE (via Lance append + delete)

Read current:
  Expr: col("mailbox_id").eq(lit(mailbox_id))
          .and(col("uid").eq(lit(5)))
  
Modify flags array, write new row, delete old row
```

### EXPUNGE

```
IMAP: EXPUNGE

Find deleted:
  Expr: col("mailbox_id").eq(lit(mailbox_id))
          .and(array_has(col("flags"), lit("\\Deleted")))

Delete matching rows from Lance
Return deleted UIDs
```

---

## Search Builder API

```rust
// Fluent builder
let search = Search::new()
    .from("alice")
    .subject("meeting")
    .unseen()
    .since(timestamp)
    .smaller(10_000);

// Convert to Expr
let expr = search.to_expr(&mailbox_id);

// Execute
let uids = query.search(&mailbox_id, &search).await?;
```

### Search Fields

```rust
pub struct Search {
    // Header searches
    pub from: Option<String>,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub subject: Option<String>,
    
    // Body search (requires full scan)
    pub body: Option<String>,
    
    // Date filters
    pub since: Option<i64>,      // internal_date >=
    pub before: Option<i64>,     // internal_date <
    pub on: Option<i64>,         // internal_date between day start/end
    pub sent_since: Option<i64>, // date >= (header Date)
    pub sent_before: Option<i64>,
    pub sent_on: Option<i64>,
    
    // Flag filters
    pub flags: FlagSearch,
    
    // Size filters
    pub size: SizeSearch,
    
    // UID filter
    pub uid_set: Option<Vec<UidRange>>,
}

pub struct FlagSearch {
    pub seen: Option<bool>,      // true = SEEN, false = UNSEEN
    pub flagged: Option<bool>,
    pub answered: Option<bool>,
    pub deleted: Option<bool>,
    pub draft: Option<bool>,
    pub recent: Option<bool>,
}

pub struct SizeSearch {
    pub larger: Option<i64>,
    pub smaller: Option<i64>,
}
```

---

## Column Pruning

Critical optimization: read only needed columns.

### FETCH FLAGS
```rust
columns: ["uid", "flags"]

// Lance reads:
// - flags column: Dict encoded, ~3 bits/row
// - uid column: FOR+Bitpack, ~4 bits/row
// 
// For 10K messages: ~9KB read (not 100MB of full records)
```

### FETCH ENVELOPE
```rust
columns: ["uid", "date", "from_addr", "from_name", "to_addrs",
          "cc_addrs", "bcc_addrs", "reply_to", "subject",
          "message_id", "in_reply_to"]

// Lance reads only these 11 columns
// Skips: body_ref, preview, size, etc.
```

### SEARCH
```rust
columns: ["uid"]

// Only return UIDs
// Filter pushdown means Lance only scans filtered columns
```

---

## Predicate Pushdown

DataFusion + Lance optimize together:

```
Query: SEARCH FROM "alice" SINCE 1-Jan-2025

Logical Plan:
  Filter: from_addr LIKE '%alice%' AND internal_date >= 1735689600000
    Scan: messages (columns: [uid, from_addr, internal_date])

Physical Plan:
  Lance pushes filter into scan
  Only reads matching row groups
  Decompresses only matching blocks
```

---

## Array Functions

For List<Utf8> columns (flags, to_addrs, etc.):

```rust
// Check if array contains value
array_has(col("flags"), lit("\\Seen"))

// Check if array contains any of values
array_has_any(col("flags"), lit(vec!["\\Seen", "\\Flagged"]))

// Check if array contains all of values
array_has_all(col("flags"), lit(vec!["\\Seen", "\\Answered"]))

// Convert array to string for LIKE
array_to_string(col("to_addrs"), lit(",")).like(lit("%alice%"))
```

---

## Performance Expectations

| Operation | Traditional (KV) | Spear (Columnar) |
|-----------|------------------|------------------|
| FETCH FLAGS (10K msgs) | 100ms (deserialize all) | <1ms (read 1 column) |
| SEARCH FROM (100K msgs) | 500ms (scan all blobs) | <50ms (scan 1 column) |
| SEARCH UNSEEN (100K msgs) | 500ms | <10ms (Dict encoded) |
| FETCH BODY | 10ms | 10ms (same - read content) |

The win is metadata operations. Body fetches are I/O bound regardless.

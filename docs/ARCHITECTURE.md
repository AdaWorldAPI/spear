# Spear Architecture Specification

## Vision

Columnar mail server built on Lance format. Not a KV adapter over Lance — proper columnar storage where IMAP operations map to typed column access.

## Why Columnar Mail?

### Traditional (Dovecot, Stalwart, Cyrus)

```
KV Store: message_id → serialized_blob

FETCH FLAGS:
1. Read entire blob (10KB average)
2. Deserialize
3. Extract flags field
4. Discard rest

SEARCH FROM alice:
1. Iterate all messages
2. Deserialize each (10KB × N)
3. Check from field
4. Return matching UIDs
```

### Spear (Columnar)

```
Lance Table: 24 typed columns

FETCH FLAGS:
1. Read flags column only
2. Dict encoded: 3 bits per row
3. No deserialization

SEARCH FROM alice:
1. Read from_addr column only
2. FSST compressed strings
3. Predicate pushdown to Lance
4. Return matching UIDs
```

## Tech Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Protocol | Stalwart crates (imap-proto, jmap-proto) | Battle-tested parsers |
| Query | DataFusion | Expr API, predicate pushdown |
| Storage | Lance | Columnar, O(1) point lookup |
| Compression | BtrBlocks-style (via Lance) | Adaptive per-column encoding |
| Bodies | Content-addressed files | SHA256 deduplication |

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT                                         │
│                         (Thunderbird, etc.)                                 │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ IMAP / JMAP / CalDAV / CardDAV
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PROTOCOL LAYER                                    │
│                                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │ imap-proto  │  │ jmap-proto  │  │  dav-proto  │  │ smtp-proto  │       │
│  │ (Stalwart)  │  │ (Stalwart)  │  │ (Stalwart)  │  │ (Stalwart)  │       │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Parsed commands
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            QUERY LAYER                                      │
│                                                                             │
│  IMAP SELECT INBOX                                                         │
│    → col("account_id").eq(lit(id)).and(col("path").eq(lit("INBOX")))       │
│                                                                             │
│  IMAP FETCH 5 (FLAGS)                                                      │
│    → col("mailbox_id").eq(lit(id)).and(col("uid").eq(lit(5)))              │
│    → select: ["flags"]  (read ONE column)                                  │
│                                                                             │
│  IMAP SEARCH FROM alice UNSEEN                                             │
│    → col("mailbox_id").eq(lit(id))                                         │
│        .and(col("from_addr").like(lit("%alice%")))                         │
│        .and(array_has(col("flags"), lit("\\Seen")).not())                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ DataFusion Expr
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DATAFUSION                                        │
│                                                                             │
│  • Logical plan optimization                                               │
│  • Predicate pushdown to Lance                                             │
│  • Column pruning (read only needed columns)                               │
│  • Parallel execution                                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Arrow RecordBatch
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           LANCE TABLES                                      │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ messages.lance                                                       │  │
│  │ ┌─────────┬─────┬────────┬──────────┬─────────┬────────┬──────────┐ │  │
│  │ │mailbox  │ uid │ date   │ from     │ subject │ flags  │ body_ref │ │  │
│  │ │─────────│─────│────────│──────────│─────────│────────│──────────│ │  │
│  │ │ Dict    │ FOR │ Delta  │ FSST     │ FSST    │ Dict   │ Dict     │ │  │
│  │ │ ~4 bits │ ~4b │ ~8b    │ compress │ compress│ ~3b    │ dedup    │ │  │
│  │ └─────────┴─────┴────────┴──────────┴─────────┴────────┴──────────┘ │  │
│  │ + B-tree index on (mailbox_id, uid)                                 │  │
│  │ + B-tree index on (account_id, internal_date)                       │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │folders.lance│  │accounts.lance│ │ events.lance│  │contacts.lance│       │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ body_ref (SHA256)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CONTENT STORE                                     │
│                                                                             │
│  content/                                                                  │
│  ├── ab/cd/abcdef1234567890...  (RFC822 message body)                     │
│  ├── ef/01/ef0123456789abcd...  (attachment)                              │
│  └── ...                                                                   │
│                                                                             │
│  • Content-addressed (SHA256 → path)                                       │
│  • Automatic deduplication                                                 │
│  • Sharded directories (ab/cd/...)                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Compression Analysis (BtrBlocks-style)

For 100K messages in a typical mailbox:

| Column | Type | Cardinality | Encoding | Bits/Row | Total |
|--------|------|-------------|----------|----------|-------|
| mailbox_id | Binary(16) | ~10 | Dict | 4 | 50KB |
| uid | UInt32 | 100K | FOR+Bitpack | 4 | 50KB |
| internal_date | Timestamp | High | Delta | 8 | 100KB |
| flags | List<Utf8> | 5 values | Dict | 3 | 37KB |
| from_addr | Utf8 | High | FSST | ~80 | 1MB |
| subject | Utf8 | High | FSST | ~120 | 1.5MB |
| body_ref | Utf8 | ~80K | Dict | 17 | 212KB |

**Total metadata: ~3MB for 100K messages** (vs ~1GB raw)

## Lance vs Parquet

| Feature | Parquet | Lance |
|---------|---------|-------|
| Columnar | ✓ | ✓ |
| Compression | ✓ | ✓ (BtrBlocks-style) |
| Point lookup | Slow (read page) | O(1) |
| Row groups | Required | None |
| Versioning | No | Git-style |
| Updates | Rewrite file | Append fragment |

## Indexes

Lance supports B-tree indexes for fast point lookups:

1. **Primary**: `(mailbox_id, uid)` - IMAP FETCH by UID
2. **Recent**: `(account_id, internal_date DESC)` - Latest mail
3. **Thread**: `thread_id` - Conversation view
4. **Folder**: `(account_id, path)` - Mailbox lookup

## Protocol Support

| Protocol | Status | Notes |
|----------|--------|-------|
| IMAP4rev2 | Planned | Via imap-proto |
| JMAP | Planned | Via jmap-proto |
| CalDAV | Planned | events.lance |
| CardDAV | Planned | contacts.lance |
| WebDAV | Planned | files.lance |
| SMTP | Future | Inbound delivery |

# Spear

Columnar mail server built on Lance.

## Why Columnar?

Traditional mail servers (Dovecot, Stalwart, Cyrus) use KV stores. Every message is a serialized blob. Want just the flags? Deserialize the whole thing.

Spear stores mail as **typed columns**:

```
messages.lance
┌─────────┬─────┬────────────┬──────────┬─────────┬────────┐
│ mailbox │ uid │ date       │ from     │ subject │ flags  │
│ Utf8    │ U32 │ Timestamp  │ Utf8     │ Utf8    │ List   │
├─────────┼─────┼────────────┼──────────┼─────────┼────────┤
│ Dict    │ FOR │ Delta      │ FSST     │ FSST    │ Dict   │
│ 1 bit   │ 4b  │ 8 bits     │ compress │ compress│ 3 bits │
└─────────┴─────┴────────────┴──────────┴─────────┴────────┘
```

### BtrBlocks-style Compression

Lance applies adaptive encoding per column:
- `mailbox`: Dict encoded (90% = "INBOX" → 1 bit per row)
- `uid`: FOR + Bitpacking (sequential → ~4 bits)
- `date`: Delta encoding (clustered by arrival → ~8 bits)
- `flags`: Dict encoded (5 standard flags → 3 bits)
- `from`, `subject`: FSST string compression

### Fast Point Lookups

Unlike Parquet, Lance supports O(1) random access via structural encoding. No page read amplification.

### SQL Queries

IMAP commands translate directly to SQL:

```
IMAP                         SQL
─────────────────────────────────────────────────────────────
SELECT INBOX              →  SELECT * FROM folders 
                             WHERE account_id=? AND path='INBOX'

FETCH 5 (FLAGS)           →  SELECT flags FROM messages
                             WHERE mailbox_id=? AND uid=5

SEARCH FROM "alice"       →  SELECT uid FROM messages 
                             WHERE mailbox_id=? 
                               AND from_addr LIKE '%alice%'

SEARCH UNSEEN SINCE 1-Jan →  SELECT uid FROM messages
                             WHERE mailbox_id=?
                               AND NOT array_contains(flags, '\Seen')
                               AND internal_date >= '2025-01-01'
```

## Architecture

```
Protocol (Stalwart crates)
    │
    ▼
Query Layer (IMAP/JMAP → SQL)
    │
    ▼
DataFusion (SQL engine)
    │
    ▼
Lance Tables (columnar, B-tree indexed)
├── messages.lance  (24 columns)
├── folders.lance   (13 columns)
├── accounts.lance  (10 columns)
├── events.lance    (CalDAV)
└── contacts.lance  (CardDAV)
    │
    ▼
content/ (SHA256-addressed bodies)
```

## Tables

### messages.lance

| Column | Type | Encoding | Index |
|--------|------|----------|-------|
| id | Binary(16) | - | Primary |
| account_id | Binary(16) | Dict | B-tree |
| mailbox_id | Binary(16) | Dict | B-tree |
| uid | UInt32 | FOR+Bitpack | B-tree |
| internal_date | Timestamp | Delta | B-tree |
| date | Timestamp | Delta | - |
| from_addr | Utf8 | FSST | - |
| subject | Utf8 | FSST | - |
| flags | List<Utf8> | Dict | - |
| body_ref | Utf8 | Dict | - |
| ... | ... | ... | ... |

### folders.lance

| Column | Type | Notes |
|--------|------|-------|
| id | Binary(16) | Primary |
| account_id | Binary(16) | B-tree |
| path | Utf8 | B-tree |
| uidvalidity | UInt32 | - |
| uidnext | UInt32 | - |
| message_count | UInt32 | - |

## Usage

```rust
use spear::Spear;

let db = Spear::open("./data").await?;

// Query messages
let messages = db.messages().await?;
let query = MessageQuery::new(&messages);

// IMAP FETCH
let batch = query.fetch_by_uid(&mailbox_id, 5, &["flags", "subject"]).await?;

// IMAP SEARCH
let uids = query.search(&mailbox_id, SearchCriteria {
    from: Some("alice".into()),
    unseen: true,
    ..Default::default()
}).await?;

// Store body
let hash = db.content.put(raw_message).await?;
```

## License

MIT OR Apache-2.0

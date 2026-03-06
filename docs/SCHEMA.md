# Spear Schema Specification

## Design Principles

1. **Typed columns** - No opaque blobs. Every field has a proper Arrow type.
2. **Encoding-aware** - Schema designed for BtrBlocks-style compression.
3. **Index-friendly** - Columns ordered for compound index efficiency.
4. **Separation** - Metadata in Lance, bodies in content store.

---

## messages.lance

Email messages. Core table.

| # | Column | Type | Nullable | Encoding | Index |
|---|--------|------|----------|----------|-------|
| 0 | id | FixedSizeBinary(16) | No | - | PK |
| 1 | account_id | FixedSizeBinary(16) | No | Dict | B-tree |
| 2 | mailbox_id | FixedSizeBinary(16) | No | Dict | B-tree |
| 3 | uid | UInt32 | No | FOR+Bitpack | B-tree |
| 4 | internal_date | Timestamp(ms, UTC) | No | Delta | B-tree |
| 5 | date | Timestamp(ms, UTC) | Yes | Delta | - |
| 6 | message_id | Utf8 | Yes | FSST | - |
| 7 | in_reply_to | Utf8 | Yes | FSST | - |
| 8 | references | List<Utf8> | Yes | FSST | - |
| 9 | from_addr | Utf8 | Yes | FSST | - |
| 10 | from_name | Utf8 | Yes | FSST | - |
| 11 | sender | Utf8 | Yes | FSST | - |
| 12 | to_addrs | List<Utf8> | Yes | FSST | - |
| 13 | cc_addrs | List<Utf8> | Yes | FSST | - |
| 14 | bcc_addrs | List<Utf8> | Yes | FSST | - |
| 15 | reply_to | List<Utf8> | Yes | FSST | - |
| 16 | subject | Utf8 | Yes | FSST | - |
| 17 | flags | List<Utf8> | No | Dict | - |
| 18 | keywords | List<Utf8> | Yes | Dict | - |
| 19 | size | Int64 | No | FOR+Bitpack | - |
| 20 | thread_id | FixedSizeBinary(16) | Yes | - | B-tree |
| 21 | body_ref | Utf8 | No | Dict | - |
| 22 | preview | Utf8 | Yes | FSST | - |
| 23 | has_attachments | Boolean | No | RLE | - |

### Encoding Rationale

- **account_id, mailbox_id**: Low cardinality (few accounts, ~10 mailboxes) → Dict
- **uid**: Sequential within mailbox → FOR (Frame of Reference) + Bitpacking
- **internal_date**: Clustered by arrival time → Delta encoding
- **flags**: 5 standard values (\Seen, \Answered, \Flagged, \Deleted, \Draft) → Dict
- **from_addr, subject**: High cardinality strings → FSST string compression
- **body_ref**: ~80% unique (dedup), 20% shared → Dict with moderate savings

### Indexes

```
PRIMARY: (mailbox_id, uid)
  - IMAP FETCH by sequence/UID
  - O(1) point lookup via Lance structural encoding

RECENT: (account_id, internal_date DESC)
  - "Show latest mail"
  - Efficient range scan

THREAD: (thread_id)
  - Conversation view
  - Group related messages
```

---

## folders.lance

Mailboxes/folders.

| # | Column | Type | Nullable | Encoding |
|---|--------|------|----------|----------|
| 0 | id | FixedSizeBinary(16) | No | - |
| 1 | account_id | FixedSizeBinary(16) | No | Dict |
| 2 | parent_id | FixedSizeBinary(16) | Yes | Dict |
| 3 | name | Utf8 | No | Dict |
| 4 | path | Utf8 | No | FSST |
| 5 | delimiter | Utf8 | No | Dict |
| 6 | flags | List<Utf8> | No | Dict |
| 7 | special_use | Utf8 | Yes | Dict |
| 8 | uidvalidity | UInt32 | No | FOR |
| 9 | uidnext | UInt32 | No | FOR |
| 10 | message_count | UInt32 | No | FOR |
| 11 | unseen_count | UInt32 | No | FOR |
| 12 | recent_count | UInt32 | No | FOR |

### Special Use Values (RFC 6154)

- `\All` - All messages
- `\Archive` - Archived messages
- `\Drafts` - Draft messages
- `\Flagged` - Flagged messages
- `\Junk` - Spam
- `\Sent` - Sent messages
- `\Trash` - Deleted messages

---

## accounts.lance

User accounts.

| # | Column | Type | Nullable | Encoding |
|---|--------|------|----------|----------|
| 0 | id | FixedSizeBinary(16) | No | - |
| 1 | username | Utf8 | No | Dict |
| 2 | password_hash | Binary | No | - |
| 3 | email | Utf8 | No | FSST |
| 4 | display_name | Utf8 | Yes | FSST |
| 5 | quota_bytes | Int64 | No | FOR |
| 6 | used_bytes | Int64 | No | FOR |
| 7 | created_at | Timestamp(ms, UTC) | No | Delta |
| 8 | last_login | Timestamp(ms, UTC) | Yes | Delta |
| 9 | active | Boolean | No | RLE |

---

## events.lance

Calendar events (CalDAV).

| # | Column | Type | Nullable | Encoding |
|---|--------|------|----------|----------|
| 0 | id | FixedSizeBinary(16) | No | - |
| 1 | account_id | FixedSizeBinary(16) | No | Dict |
| 2 | calendar_id | FixedSizeBinary(16) | No | Dict |
| 3 | uid | Utf8 | No | FSST |
| 4 | summary | Utf8 | Yes | FSST |
| 5 | description | Utf8 | Yes | FSST |
| 6 | location | Utf8 | Yes | FSST |
| 7 | start_time | Timestamp(ms, UTC) | No | Delta |
| 8 | end_time | Timestamp(ms, UTC) | No | Delta |
| 9 | all_day | Boolean | No | RLE |
| 10 | timezone | Utf8 | Yes | Dict |
| 11 | recurrence_rule | Utf8 | Yes | FSST |
| 12 | recurrence_id | Timestamp(ms, UTC) | Yes | Delta |
| 13 | organizer | Utf8 | Yes | FSST |
| 14 | attendees | List<Utf8> | Yes | FSST |
| 15 | status | Utf8 | Yes | Dict |
| 16 | transparency | Utf8 | Yes | Dict |
| 17 | categories | List<Utf8> | Yes | Dict |
| 18 | created_at | Timestamp(ms, UTC) | No | Delta |
| 19 | modified_at | Timestamp(ms, UTC) | No | Delta |
| 20 | etag | Utf8 | No | - |

---

## contacts.lance

Address book contacts (CardDAV).

| # | Column | Type | Nullable | Encoding |
|---|--------|------|----------|----------|
| 0 | id | FixedSizeBinary(16) | No | - |
| 1 | account_id | FixedSizeBinary(16) | No | Dict |
| 2 | addressbook_id | FixedSizeBinary(16) | No | Dict |
| 3 | uid | Utf8 | No | FSST |
| 4 | display_name | Utf8 | Yes | FSST |
| 5 | first_name | Utf8 | Yes | FSST |
| 6 | last_name | Utf8 | Yes | FSST |
| 7 | nickname | Utf8 | Yes | FSST |
| 8 | emails | List<Utf8> | Yes | FSST |
| 9 | phones | List<Utf8> | Yes | FSST |
| 10 | addresses | List<Utf8> | Yes | FSST |
| 11 | organization | Utf8 | Yes | FSST |
| 12 | title | Utf8 | Yes | FSST |
| 13 | notes | Utf8 | Yes | FSST |
| 14 | photo_ref | Utf8 | Yes | Dict |
| 15 | birthday | Date32 | Yes | Delta |
| 16 | categories | List<Utf8> | Yes | Dict |
| 17 | created_at | Timestamp(ms, UTC) | No | Delta |
| 18 | modified_at | Timestamp(ms, UTC) | No | Delta |
| 19 | etag | Utf8 | No | - |

---

## Content Store

Not a Lance table. File-based content-addressed storage.

```
content/
├── ab/
│   └── cd/
│       └── abcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678
└── ef/
    └── 01/
        └── ef0123456789abcdef0123456789abcdef0123456789abcdef0123456789ab
```

### Design

- **Path**: `content/{hash[0:2]}/{hash[2:4]}/{hash}`
- **Hash**: SHA256 of content
- **Deduplication**: Same content = same hash = stored once
- **Atomic writes**: Write to .tmp, rename

### What Goes Here

- RFC822 message bodies (referenced by messages.body_ref)
- Attachments (if stored separately)
- Contact photos (referenced by contacts.photo_ref)
- Calendar attachments

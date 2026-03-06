//! Messages Schema
//!
//! Columnar layout optimized for email access patterns.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │ Column          │ Type           │ Encoding        │ Index              │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │ id              │ Binary(16)     │ -               │ Primary            │
//! │ account_id      │ Binary(16)     │ Dict (few)      │ B-tree (compound)  │
//! │ mailbox_id      │ Binary(16)     │ Dict (few)      │ B-tree (compound)  │
//! │ uid             │ UInt32         │ FOR+Bitpack     │ B-tree (compound)  │
//! │ internal_date   │ Timestamp      │ Delta           │ B-tree             │
//! │ date            │ Timestamp      │ Delta           │ -                  │
//! │ message_id      │ Utf8           │ FSST            │ -                  │
//! │ in_reply_to     │ Utf8           │ FSST            │ -                  │
//! │ references      │ List<Utf8>     │ FSST            │ -                  │
//! │ from_addr       │ Utf8           │ FSST            │ -                  │
//! │ from_name       │ Utf8           │ FSST            │ -                  │
//! │ sender          │ Utf8           │ FSST            │ -                  │
//! │ to_addrs        │ List<Utf8>     │ FSST            │ -                  │
//! │ cc_addrs        │ List<Utf8>     │ FSST            │ -                  │
//! │ bcc_addrs       │ List<Utf8>     │ FSST            │ -                  │
//! │ reply_to        │ List<Utf8>     │ FSST            │ -                  │
//! │ subject         │ Utf8           │ FSST            │ -                  │
//! │ flags           │ List<Utf8>     │ Dict (5 common) │ -                  │
//! │ keywords        │ List<Utf8>     │ Dict            │ -                  │
//! │ size            │ Int64          │ FOR+Bitpack     │ -                  │
//! │ thread_id       │ Binary(16)     │ -               │ B-tree             │
//! │ body_ref        │ Utf8           │ Dict (dedup)    │ -                  │
//! │ preview         │ Utf8           │ FSST            │ -                  │
//! │ has_attachments │ Boolean        │ RLE             │ -                  │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Indexes (B-tree via Lance)
//!
//! 1. **Primary**: (mailbox_id, uid) - IMAP FETCH by UID
//! 2. **Recent**: (account_id, internal_date DESC) - "latest mail"
//! 3. **Thread**: thread_id - conversation view
//!
//! ## Compression Analysis
//!
//! For 100K messages:
//! - mailbox_id: Dict → ~1 bit per row (10 mailboxes = 4 bits)
//! - uid: FOR+Bitpack → ~4 bits (sequential within mailbox)
//! - flags: Dict → ~3 bits (5 standard flags)
//! - internal_date: Delta → ~8 bits (clustered by arrival)
//! - from_addr: FSST → ~40% compression on email addresses

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use std::sync::{Arc, LazyLock};

pub static MESSAGES_SCHEMA: LazyLock<Schema> = LazyLock::new(schema);

pub fn schema() -> Schema {
    Schema::new(vec![
        // Identity
        Field::new("id", DataType::FixedSizeBinary(16), false),
        Field::new("account_id", DataType::FixedSizeBinary(16), false),
        Field::new("mailbox_id", DataType::FixedSizeBinary(16), false),
        Field::new("uid", DataType::UInt32, false),
        
        // Timestamps
        Field::new("internal_date", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("date", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), true),
        
        // Message-ID headers
        Field::new("message_id", DataType::Utf8, true),
        Field::new("in_reply_to", DataType::Utf8, true),
        Field::new("references", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        
        // Addresses
        Field::new("from_addr", DataType::Utf8, true),
        Field::new("from_name", DataType::Utf8, true),
        Field::new("sender", DataType::Utf8, true),
        Field::new("to_addrs", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("cc_addrs", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("bcc_addrs", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("reply_to", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        
        // Subject
        Field::new("subject", DataType::Utf8, true),
        
        // Flags (IMAP: \Seen, \Answered, \Flagged, \Deleted, \Draft)
        Field::new("flags", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), false),
        
        // Keywords (custom flags)
        Field::new("keywords", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        
        // Size
        Field::new("size", DataType::Int64, false),
        
        // Threading
        Field::new("thread_id", DataType::FixedSizeBinary(16), true),
        
        // Content reference (SHA256 hash)
        Field::new("body_ref", DataType::Utf8, false),
        
        // Preview text (first ~200 chars)
        Field::new("preview", DataType::Utf8, true),
        
        // Quick filter
        Field::new("has_attachments", DataType::Boolean, false),
    ])
}

/// Column indices for direct access
pub mod col {
    pub const ID: usize = 0;
    pub const ACCOUNT_ID: usize = 1;
    pub const MAILBOX_ID: usize = 2;
    pub const UID: usize = 3;
    pub const INTERNAL_DATE: usize = 4;
    pub const DATE: usize = 5;
    pub const MESSAGE_ID: usize = 6;
    pub const IN_REPLY_TO: usize = 7;
    pub const REFERENCES: usize = 8;
    pub const FROM_ADDR: usize = 9;
    pub const FROM_NAME: usize = 10;
    pub const SENDER: usize = 11;
    pub const TO_ADDRS: usize = 12;
    pub const CC_ADDRS: usize = 13;
    pub const BCC_ADDRS: usize = 14;
    pub const REPLY_TO: usize = 15;
    pub const SUBJECT: usize = 16;
    pub const FLAGS: usize = 17;
    pub const KEYWORDS: usize = 18;
    pub const SIZE: usize = 19;
    pub const THREAD_ID: usize = 20;
    pub const BODY_REF: usize = 21;
    pub const PREVIEW: usize = 22;
    pub const HAS_ATTACHMENTS: usize = 23;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_schema_valid() {
        let s = schema();
        assert_eq!(s.fields().len(), 24);
        assert_eq!(s.field(col::ID).name(), "id");
        assert_eq!(s.field(col::FLAGS).name(), "flags");
        assert_eq!(s.field(col::BODY_REF).name(), "body_ref");
    }
}

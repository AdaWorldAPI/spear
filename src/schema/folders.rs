//! Folders Schema (Mailboxes)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │ Column          │ Type           │ Encoding        │ Index              │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │ id              │ Binary(16)     │ -               │ Primary            │
//! │ account_id      │ Binary(16)     │ Dict            │ B-tree             │
//! │ parent_id       │ Binary(16)     │ Dict (sparse)   │ -                  │
//! │ name            │ Utf8           │ Dict            │ -                  │
//! │ path            │ Utf8           │ FSST            │ B-tree             │
//! │ delimiter       │ Utf8           │ Dict (/ or .)   │ -                  │
//! │ flags           │ List<Utf8>     │ Dict            │ -                  │
//! │ special_use     │ Utf8           │ Dict            │ -                  │
//! │ uidvalidity     │ UInt32         │ FOR             │ -                  │
//! │ uidnext         │ UInt32         │ FOR             │ -                  │
//! │ message_count   │ UInt32         │ FOR             │ -                  │
//! │ unseen_count    │ UInt32         │ FOR             │ -                  │
//! │ recent_count    │ UInt32         │ FOR             │ -                  │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Special Use Flags (RFC 6154)
//!
//! - \All, \Archive, \Drafts, \Flagged, \Junk, \Sent, \Trash

use arrow::datatypes::{DataType, Field, Schema};
use std::sync::{Arc, LazyLock};

pub static FOLDERS_SCHEMA: LazyLock<Schema> = LazyLock::new(schema);

pub fn schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(16), false),
        Field::new("account_id", DataType::FixedSizeBinary(16), false),
        Field::new("parent_id", DataType::FixedSizeBinary(16), true),
        Field::new("name", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("delimiter", DataType::Utf8, false),
        Field::new("flags", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), false),
        Field::new("special_use", DataType::Utf8, true),
        Field::new("uidvalidity", DataType::UInt32, false),
        Field::new("uidnext", DataType::UInt32, false),
        Field::new("message_count", DataType::UInt32, false),
        Field::new("unseen_count", DataType::UInt32, false),
        Field::new("recent_count", DataType::UInt32, false),
    ])
}

pub mod col {
    pub const ID: usize = 0;
    pub const ACCOUNT_ID: usize = 1;
    pub const PARENT_ID: usize = 2;
    pub const NAME: usize = 3;
    pub const PATH: usize = 4;
    pub const DELIMITER: usize = 5;
    pub const FLAGS: usize = 6;
    pub const SPECIAL_USE: usize = 7;
    pub const UIDVALIDITY: usize = 8;
    pub const UIDNEXT: usize = 9;
    pub const MESSAGE_COUNT: usize = 10;
    pub const UNSEEN_COUNT: usize = 11;
    pub const RECENT_COUNT: usize = 12;
}

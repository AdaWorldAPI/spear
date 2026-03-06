//! Query Layer
//!
//! DataFusion expressions over Lance tables.
//!
//! ```text
//! IMAP Command              │ DataFusion Expression
//! ──────────────────────────┼────────────────────────────────────────────────
//! SELECT INBOX              │ col("path").eq(lit("INBOX"))
//! ──────────────────────────┼────────────────────────────────────────────────
//! FETCH 1:* FLAGS           │ col("mailbox_id").eq(lit(id))
//!                           │ → select only "uid", "flags" columns
//! ──────────────────────────┼────────────────────────────────────────────────
//! FETCH 5 (ENVELOPE)        │ col("mailbox_id").eq(lit(id))
//!                           │   .and(col("uid").eq(lit(5)))
//!                           │ → select envelope columns only
//! ──────────────────────────┼────────────────────────────────────────────────
//! SEARCH FROM "alice"       │ col("mailbox_id").eq(lit(id))
//!                           │   .and(col("from_addr").like(lit("%alice%")))
//! ──────────────────────────┼────────────────────────────────────────────────
//! SEARCH UNSEEN             │ col("mailbox_id").eq(lit(id))
//!                           │   .and(array_has(col("flags"), lit("\Seen")).not())
//! ──────────────────────────┼────────────────────────────────────────────────
//! SEARCH SINCE 1-Jan-2025   │ col("mailbox_id").eq(lit(id))
//!                           │   .and(col("internal_date").gt_eq(lit(ts)))
//! ```
//!
//! ## Why DataFusion Expressions (not SQL strings)
//!
//! - Type-safe: compiler catches mistakes
//! - Composable: build complex queries programmatically
//! - Optimizable: DataFusion can push predicates into Lance
//! - No escaping: no SQL injection, no quoting hell

pub mod imap;
pub mod messages;

pub use messages::{MessageQuery, Search, FlagSearch, SizeSearch};

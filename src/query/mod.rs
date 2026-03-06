//! Query Layer — DataFusion DataFrame API
//!
//! No SQL strings. Type-safe predicate pushdown to Lance.
//!
//! ```text
//! IMAP Command              │ DataFusion DataFrame
//! ──────────────────────────┼──────────────────────────────────────────
//! SELECT INBOX              │ ctx.table("folders")
//!                           │    .filter(col("account_id").eq(lit(...)))
//!                           │    .filter(col("path").eq(lit("INBOX")))
//! ──────────────────────────┼──────────────────────────────────────────
//! FETCH 1:* FLAGS           │ ctx.table("messages")
//!                           │    .filter(col("mailbox_id").eq(lit(...)))
//!                           │    .select_columns(&["uid", "flags"])
//!                           │    .sort(vec![col("uid").sort(true, false)])
//! ──────────────────────────┼──────────────────────────────────────────
//! FETCH 5 (ENVELOPE)        │ ctx.table("messages")
//!                           │    .filter(col("mailbox_id").eq(lit(...))
//!                           │        .and(col("uid").eq(lit(5))))
//!                           │    .select_columns(&["date", "subject",
//!                           │        "from_addr", "to_addrs", ...])
//! ──────────────────────────┼──────────────────────────────────────────
//! SEARCH FROM "alice"       │ ctx.table("messages")
//!                           │    .filter(col("mailbox_id").eq(lit(...))
//!                           │        .and(col("from_addr").like(lit("%alice%"))))
//!                           │    .select_columns(&["uid"])
//! ──────────────────────────┼──────────────────────────────────────────
//! SEARCH UNSEEN             │ ctx.table("messages")
//!                           │    .filter(col("mailbox_id").eq(lit(...))
//!                           │        .and(array_has(col("flags"), lit("\\Seen")).not()))
//!                           │    .select_columns(&["uid"])
//! ──────────────────────────┼──────────────────────────────────────────
//! SEARCH SINCE 1-Jan-2025   │ ctx.table("messages")
//!                           │    .filter(col("internal_date").gt_eq(lit(timestamp)))
//! ```
//!
//! ## Why DataFusion?
//!
//! - Type-safe expression building (no SQL injection)
//! - Predicate pushdown to Lance columnar format
//! - Only reads requested columns (projection pushdown)
//! - Composable filters via .and() / .or()
//! - Native Arrow types throughout

pub mod imap;
pub mod messages;

pub use messages::{MessageQuery, SearchCriteria};

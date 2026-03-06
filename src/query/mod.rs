//! Query Layer
//!
//! Translates IMAP/JMAP commands to SQL over Lance tables.
//!
//! ```text
//! IMAP Command              │ SQL Query
//! ──────────────────────────┼────────────────────────────────────────────────
//! SELECT INBOX              │ SELECT * FROM folders 
//!                           │ WHERE account_id=? AND path='INBOX'
//! ──────────────────────────┼────────────────────────────────────────────────
//! FETCH 1:* FLAGS           │ SELECT uid, flags FROM messages
//!                           │ WHERE mailbox_id=? ORDER BY uid
//! ──────────────────────────┼────────────────────────────────────────────────
//! FETCH 5 (ENVELOPE)        │ SELECT uid, date, subject, from_addr, from_name,
//!                           │        to_addrs, cc_addrs, bcc_addrs, reply_to,
//!                           │        message_id, in_reply_to
//!                           │ FROM messages WHERE mailbox_id=? AND uid=5
//! ──────────────────────────┼────────────────────────────────────────────────
//! FETCH 5 BODY[]            │ SELECT body_ref FROM messages 
//!                           │ WHERE mailbox_id=? AND uid=5
//!                           │ → then fetch from content store
//! ──────────────────────────┼────────────────────────────────────────────────
//! SEARCH FROM "alice"       │ SELECT uid FROM messages 
//!                           │ WHERE mailbox_id=? AND from_addr LIKE '%alice%'
//! ──────────────────────────┼────────────────────────────────────────────────
//! SEARCH UNSEEN             │ SELECT uid FROM messages
//!                           │ WHERE mailbox_id=? 
//!                           │   AND NOT array_contains(flags, '\Seen')
//! ──────────────────────────┼────────────────────────────────────────────────
//! SEARCH SINCE 1-Jan-2025   │ SELECT uid FROM messages
//!                           │ WHERE mailbox_id=? AND internal_date >= ?
//! ──────────────────────────┼────────────────────────────────────────────────
//! STORE +FLAGS (\Seen)      │ UPDATE messages SET flags = array_append(flags, '\Seen')
//!                           │ WHERE mailbox_id=? AND uid=?
//! ──────────────────────────┼────────────────────────────────────────────────
//! EXPUNGE                   │ DELETE FROM messages
//!                           │ WHERE mailbox_id=? AND array_contains(flags, '\Deleted')
//! ```
//!
//! ## Why This is Better
//!
//! Traditional (Dovecot/Stalwart):
//! - FETCH FLAGS: Deserialize entire message record, extract flags
//! - SEARCH: Load all messages, deserialize, filter in memory
//!
//! Spear:
//! - FETCH FLAGS: Read only `flags` column (3 bits per row, Dict encoded)
//! - SEARCH: Columnar scan with predicate pushdown, no deserialization

pub mod imap;
pub mod messages;

pub use messages::MessageQuery;

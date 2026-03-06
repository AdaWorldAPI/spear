//! # Spear
//!
//! Columnar mail server built on Lance.
//!
//! ## Architecture
//!
//! ```text
//! Protocol (Stalwart crates)
//!     │
//!     ▼
//! Query Layer (IMAP/JMAP → SQL)
//!     │
//!     ▼
//! DataFusion (SQL engine)
//!     │
//!     ▼
//! Lance Tables (columnar, B-tree indexed)
//! ├── messages.lance
//! ├── folders.lance
//! ├── accounts.lance
//! ├── events.lance
//! └── contacts.lance
//!     │
//!     ▼
//! content/ (SHA256-addressed bodies)
//! ```
//!
//! ## Why Columnar?
//!
//! Traditional mail servers (Dovecot, Stalwart) use KV stores.
//! Every message = serialized blob. No structure.
//!
//! Spear stores mail as **typed columns**:
//!
//! ```text
//! messages.lance
//! ┌─────────┬─────┬────────────┬──────────┬─────────┬────────┐
//! │ mailbox │ uid │ date       │ from     │ subject │ flags  │
//! │ Utf8    │ U32 │ Timestamp  │ Utf8     │ Utf8    │ List   │
//! ├─────────┼─────┼────────────┼──────────┼─────────┼────────┤
//! │ Dict    │ FOR │ Delta      │ FSST     │ FSST    │ Dict   │
//! │ 1 bit   │ 4b  │ 8 bits     │ compress │ compress│ 3 bits │
//! └─────────┴─────┴────────────┴──────────┴─────────┴────────┘
//!
//! + B-tree index on (mailbox, uid)
//! + B-tree index on (account, date)
//! ```
//!
//! Benefits:
//! - BtrBlocks-style compression per column type
//! - O(1) point lookup via Lance structural encoding
//! - SQL queries (SEARCH FROM alice → WHERE from LIKE '%alice%')
//! - Only read columns you need (FETCH FLAGS → read 1 column)

pub mod schema;
pub mod query;
pub mod content;
pub mod error;

pub use error::{Error, Result};

use std::path::Path;
use std::sync::Arc;

/// Main database handle
pub struct Spear {
    pub db: lancedb::Connection,
    pub content: content::ContentStore,
    path: std::path::PathBuf,
}

impl Spear {
    /// Open or create database
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;
        
        let db = lancedb::connect(path.to_str().unwrap())
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        // Initialize tables with proper schemas
        schema::init_tables(&db).await?;
        
        let content = content::ContentStore::open(path.join("content"))?;
        
        Ok(Self {
            db,
            content,
            path: path.to_path_buf(),
        })
    }
    
    /// Get messages table for queries
    pub async fn messages(&self) -> Result<lancedb::Table> {
        self.db.open_table("messages")
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))
    }
    
    /// Get folders table
    pub async fn folders(&self) -> Result<lancedb::Table> {
        self.db.open_table("folders")
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))
    }
    
    /// Get accounts table
    pub async fn accounts(&self) -> Result<lancedb::Table> {
        self.db.open_table("accounts")
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))
    }
}

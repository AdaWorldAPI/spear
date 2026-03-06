//! Columnar Schemas
//!
//! Each table is a proper columnar structure with typed fields.
//! Lance applies BtrBlocks-style encoding per column:
//!
//! - Utf8 with low cardinality → Dictionary encoding
//! - Utf8 with high cardinality → FSST string compression
//! - UInt32 sequential → FOR + Bitpacking
//! - Timestamp clustered → Delta encoding
//! - List<Utf8> with few values → Dictionary encoding

pub mod messages;
pub mod folders;
pub mod accounts;
pub mod events;
pub mod contacts;

pub use messages::MESSAGES_SCHEMA;
pub use folders::FOLDERS_SCHEMA;
pub use accounts::ACCOUNTS_SCHEMA;

use crate::error::{Error, Result};
use arrow::record_batch::RecordBatch;
use lancedb::Connection;
use std::sync::Arc;

/// Initialize all tables with proper schemas
pub async fn init_tables(db: &Connection) -> Result<()> {
    // Messages
    if db.open_table("messages").execute().await.is_err() {
        let batch = RecordBatch::new_empty(Arc::new(messages::schema()));
        db.create_table("messages", batch)
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        // TODO: Create B-tree indexes when LanceDB API supports it
        // - (mailbox_id, uid) - primary access
        // - (account_id, internal_date) - recent mail
        // - thread_id - threading
    }
    
    // Folders
    if db.open_table("folders").execute().await.is_err() {
        let batch = RecordBatch::new_empty(Arc::new(folders::schema()));
        db.create_table("folders", batch)
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        // TODO: Index on (account_id, path)
    }
    
    // Accounts
    if db.open_table("accounts").execute().await.is_err() {
        let batch = RecordBatch::new_empty(Arc::new(accounts::schema()));
        db.create_table("accounts", batch)
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        // TODO: Index on username
    }
    
    // Events (calendar)
    if db.open_table("events").execute().await.is_err() {
        let batch = RecordBatch::new_empty(Arc::new(events::schema()));
        db.create_table("events", batch)
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
    }
    
    // Contacts
    if db.open_table("contacts").execute().await.is_err() {
        let batch = RecordBatch::new_empty(Arc::new(contacts::schema()));
        db.create_table("contacts", batch)
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
    }
    
    Ok(())
}

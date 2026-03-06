//! Message Queries
//!
//! SQL queries over messages.lance via DataFusion.

use crate::error::{Error, Result};
use crate::schema::messages::col;
use arrow::array::*;
use arrow::record_batch::RecordBatch;
use futures::TryStreamExt;
use lancedb::Table;

/// Message query builder
pub struct MessageQuery<'a> {
    table: &'a Table,
}

impl<'a> MessageQuery<'a> {
    pub fn new(table: &'a Table) -> Self {
        Self { table }
    }
    
    /// IMAP FETCH by mailbox + UID
    pub async fn fetch_by_uid(
        &self,
        mailbox_id: &[u8; 16],
        uid: u32,
        columns: &[&str],
    ) -> Result<Option<RecordBatch>> {
        let mailbox_hex = hex(mailbox_id);
        let filter = format!(
            "mailbox_id = X'{}' AND uid = {}",
            mailbox_hex, uid
        );
        
        let query = self.table.query()
            .select(lancedb::query::Select::Columns(
                columns.iter().map(|s| s.to_string()).collect()
            ))
            .filter(filter);
        
        let batches: Vec<RecordBatch> = query
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?
            .try_collect()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }
        
        Ok(Some(batches.into_iter().next().unwrap()))
    }
    
    /// IMAP FETCH range - only specified columns
    pub async fn fetch_range(
        &self,
        mailbox_id: &[u8; 16],
        uid_from: u32,
        uid_to: u32,
        columns: &[&str],
    ) -> Result<Vec<RecordBatch>> {
        let mailbox_hex = hex(mailbox_id);
        let filter = format!(
            "mailbox_id = X'{}' AND uid >= {} AND uid <= {}",
            mailbox_hex, uid_from, uid_to
        );
        
        let query = self.table.query()
            .select(lancedb::query::Select::Columns(
                columns.iter().map(|s| s.to_string()).collect()
            ))
            .filter(filter);
        
        let batches: Vec<RecordBatch> = query
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?
            .try_collect()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        Ok(batches)
    }
    
    /// IMAP SEARCH - returns UIDs matching criteria
    pub async fn search(
        &self,
        mailbox_id: &[u8; 16],
        criteria: SearchCriteria,
    ) -> Result<Vec<u32>> {
        let mailbox_hex = hex(mailbox_id);
        let mut filters = vec![format!("mailbox_id = X'{}'", mailbox_hex)];
        
        // Build filter from criteria
        if let Some(from) = &criteria.from {
            filters.push(format!("from_addr LIKE '%{}%'", escape_sql(from)));
        }
        if let Some(to) = &criteria.to {
            filters.push(format!("array_to_string(to_addrs, ',') LIKE '%{}%'", escape_sql(to)));
        }
        if let Some(subject) = &criteria.subject {
            filters.push(format!("subject LIKE '%{}%'", escape_sql(subject)));
        }
        if let Some(since) = criteria.since {
            filters.push(format!("internal_date >= {}", since));
        }
        if let Some(before) = criteria.before {
            filters.push(format!("internal_date < {}", before));
        }
        if criteria.unseen {
            // NOT array_contains requires custom handling
            filters.push("NOT array_contains(flags, '\\\\Seen')".to_string());
        }
        if criteria.flagged {
            filters.push("array_contains(flags, '\\\\Flagged')".to_string());
        }
        if criteria.answered {
            filters.push("array_contains(flags, '\\\\Answered')".to_string());
        }
        if criteria.deleted {
            filters.push("array_contains(flags, '\\\\Deleted')".to_string());
        }
        
        let filter = filters.join(" AND ");
        
        let query = self.table.query()
            .select(lancedb::query::Select::Columns(vec!["uid".to_string()]))
            .filter(filter);
        
        let batches: Vec<RecordBatch> = query
            .execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?
            .try_collect()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?;
        
        let mut uids = Vec::new();
        for batch in batches {
            let uid_col = batch.column(0)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .ok_or_else(|| Error::Lance("Expected UInt32".into()))?;
            
            for i in 0..uid_col.len() {
                uids.push(uid_col.value(i));
            }
        }
        
        uids.sort();
        Ok(uids)
    }
    
    /// Count messages in mailbox
    pub async fn count(&self, mailbox_id: &[u8; 16]) -> Result<u32> {
        let uids = self.search(mailbox_id, SearchCriteria::default()).await?;
        Ok(uids.len() as u32)
    }
    
    /// Get max UID in mailbox
    pub async fn max_uid(&self, mailbox_id: &[u8; 16]) -> Result<u32> {
        let uids = self.search(mailbox_id, SearchCriteria::default()).await?;
        Ok(uids.into_iter().max().unwrap_or(0))
    }
    
    /// List all UIDs in mailbox
    pub async fn list_uids(&self, mailbox_id: &[u8; 16]) -> Result<Vec<u32>> {
        self.search(mailbox_id, SearchCriteria::default()).await
    }
}

/// IMAP SEARCH criteria
#[derive(Default)]
pub struct SearchCriteria {
    pub from: Option<String>,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub since: Option<i64>,      // Timestamp milliseconds
    pub before: Option<i64>,
    pub unseen: bool,
    pub flagged: bool,
    pub answered: bool,
    pub deleted: bool,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn escape_sql(s: &str) -> String {
    s.replace('\'', "''").replace('%', "\\%").replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hex() {
        let bytes = [0x01, 0x02, 0xab, 0xcd];
        assert_eq!(hex(&bytes), "0102abcd");
    }
    
    #[test]
    fn test_escape_sql() {
        assert_eq!(escape_sql("O'Brien"), "O''Brien");
    }
}

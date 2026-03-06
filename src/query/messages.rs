//! Message Queries via DataFusion
//!
//! Uses DataFusion DataFrame API — no SQL strings.
//! Predicate pushdown to Lance for columnar efficiency.

use crate::error::{Error, Result};
use arrow::array::*;
use arrow::record_batch::RecordBatch;
use datafusion::prelude::*;
use datafusion::datasource::MemTable;
use std::sync::Arc;

/// Message query executor
pub struct MessageQuery {
    ctx: SessionContext,
}

impl MessageQuery {
    /// Create query context with messages table registered
    pub async fn new(batches: Vec<RecordBatch>) -> Result<Self> {
        let ctx = SessionContext::new();
        
        if !batches.is_empty() {
            let schema = batches[0].schema();
            let table = MemTable::try_new(schema, vec![batches])
                .map_err(|e| Error::DataFusion(e.to_string()))?;
            ctx.register_table("messages", Arc::new(table))
                .map_err(|e| Error::DataFusion(e.to_string()))?;
        }
        
        Ok(Self { ctx })
    }
    
    /// IMAP FETCH by mailbox + UID — reads only requested columns
    pub async fn fetch_by_uid(
        &self,
        mailbox_id: &[u8; 16],
        uid: u32,
        columns: &[&str],
    ) -> Result<Option<RecordBatch>> {
        let df = self.ctx.table("messages").await
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .filter(
                col("mailbox_id").eq(lit(mailbox_id.as_slice()))
                    .and(col("uid").eq(lit(uid)))
            )
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .select_columns(columns)
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        let batches = df.collect().await
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }
        
        // Combine batches into one
        let schema = batches[0].schema();
        let batch = arrow::compute::concat_batches(&schema, &batches)
            .map_err(|e| Error::Arrow(e))?;
        
        Ok(Some(batch))
    }
    
    /// IMAP FETCH range — only specified columns
    pub async fn fetch_range(
        &self,
        mailbox_id: &[u8; 16],
        uid_from: u32,
        uid_to: u32,
        columns: &[&str],
    ) -> Result<Vec<RecordBatch>> {
        let df = self.ctx.table("messages").await
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .filter(
                col("mailbox_id").eq(lit(mailbox_id.as_slice()))
                    .and(col("uid").gt_eq(lit(uid_from)))
                    .and(col("uid").lt_eq(lit(uid_to)))
            )
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .select_columns(columns)
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .sort(vec![col("uid").sort(true, false)])
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        df.collect().await
            .map_err(|e| Error::DataFusion(e.to_string()))
    }
    
    /// IMAP SEARCH — returns UIDs matching criteria
    pub async fn search(
        &self,
        mailbox_id: &[u8; 16],
        criteria: SearchCriteria,
    ) -> Result<Vec<u32>> {
        // Start with mailbox filter
        let mut expr = col("mailbox_id").eq(lit(mailbox_id.as_slice()));
        
        // Add criteria filters
        if let Some(ref from) = criteria.from {
            expr = expr.and(col("from_addr").like(lit(format!("%{}%", from))));
        }
        if let Some(ref subject) = criteria.subject {
            expr = expr.and(col("subject").like(lit(format!("%{}%", subject))));
        }
        if let Some(since) = criteria.since {
            expr = expr.and(col("internal_date").gt_eq(lit(since)));
        }
        if let Some(before) = criteria.before {
            expr = expr.and(col("internal_date").lt(lit(before)));
        }
        if criteria.unseen {
            // array_contains for flags
            expr = expr.and(
                array_has(col("flags"), lit("\\Seen")).not()
            );
        }
        if criteria.seen {
            expr = expr.and(array_has(col("flags"), lit("\\Seen")));
        }
        if criteria.flagged {
            expr = expr.and(array_has(col("flags"), lit("\\Flagged")));
        }
        if criteria.answered {
            expr = expr.and(array_has(col("flags"), lit("\\Answered")));
        }
        if criteria.deleted {
            expr = expr.and(array_has(col("flags"), lit("\\Deleted")));
        }
        if let Some(larger) = criteria.larger {
            expr = expr.and(col("size").gt(lit(larger)));
        }
        if let Some(smaller) = criteria.smaller {
            expr = expr.and(col("size").lt(lit(smaller)));
        }
        
        let df = self.ctx.table("messages").await
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .filter(expr)
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .select_columns(&["uid"])
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .sort(vec![col("uid").sort(true, false)])
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        let batches = df.collect().await
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        let mut uids = Vec::new();
        for batch in batches {
            let uid_col = batch.column(0)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .ok_or_else(|| Error::Invalid("Expected UInt32".into()))?;
            
            for i in 0..uid_col.len() {
                uids.push(uid_col.value(i));
            }
        }
        
        Ok(uids)
    }
    
    /// Count messages in mailbox
    pub async fn count(&self, mailbox_id: &[u8; 16]) -> Result<u64> {
        let df = self.ctx.table("messages").await
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .filter(col("mailbox_id").eq(lit(mailbox_id.as_slice())))
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .aggregate(vec![], vec![count(lit(1)).alias("count")])
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        let batches = df.collect().await
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(0);
        }
        
        let count_col = batches[0].column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| Error::Invalid("Expected Int64".into()))?;
        
        Ok(count_col.value(0) as u64)
    }
    
    /// Get max UID in mailbox
    pub async fn max_uid(&self, mailbox_id: &[u8; 16]) -> Result<u32> {
        let df = self.ctx.table("messages").await
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .filter(col("mailbox_id").eq(lit(mailbox_id.as_slice())))
            .map_err(|e| Error::DataFusion(e.to_string()))?
            .aggregate(vec![], vec![max(col("uid")).alias("max_uid")])
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        let batches = df.collect().await
            .map_err(|e| Error::DataFusion(e.to_string()))?;
        
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(0);
        }
        
        let max_col = batches[0].column(0)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .ok_or_else(|| Error::Invalid("Expected UInt32".into()))?;
        
        if max_col.is_null(0) {
            return Ok(0);
        }
        
        Ok(max_col.value(0))
    }
}

/// DataFusion array_has (array_contains)
fn array_has(array: Expr, element: Expr) -> Expr {
    datafusion::functions_array::expr_fn::array_has(array, element)
}

/// IMAP SEARCH criteria
#[derive(Default, Debug, Clone)]
pub struct SearchCriteria {
    pub from: Option<String>,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub on: Option<i64>,
    pub seen: bool,
    pub unseen: bool,
    pub flagged: bool,
    pub unflagged: bool,
    pub answered: bool,
    pub deleted: bool,
    pub draft: bool,
    pub larger: Option<i64>,
    pub smaller: Option<i64>,
    pub uid_set: Option<Vec<u32>>,
}

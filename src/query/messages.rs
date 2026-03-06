//! Message Queries
//!
//! DataFusion expressions over Lance tables.
//! No SQL strings. Type-safe predicates.

use crate::error::{Error, Result};
use arrow::array::*;
use arrow::record_batch::RecordBatch;
use datafusion::prelude::*;
use datafusion::logical_expr::{col, lit, Expr};
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
        let filter = mailbox_eq(mailbox_id)
            .and(col("uid").eq(lit(uid)));
        
        let batches = self.execute(columns, filter).await?;
        Ok(batches.into_iter().next())
    }
    
    /// IMAP FETCH range
    pub async fn fetch_range(
        &self,
        mailbox_id: &[u8; 16],
        uid_from: u32,
        uid_to: u32,
        columns: &[&str],
    ) -> Result<Vec<RecordBatch>> {
        let filter = mailbox_eq(mailbox_id)
            .and(col("uid").gt_eq(lit(uid_from)))
            .and(col("uid").lt_eq(lit(uid_to)));
        
        self.execute(columns, filter).await
    }
    
    /// IMAP SEARCH
    pub async fn search(
        &self,
        mailbox_id: &[u8; 16],
        criteria: &Search,
    ) -> Result<Vec<u32>> {
        let filter = criteria.to_expr(mailbox_id);
        let batches = self.execute(&["uid"], filter).await?;
        
        let mut uids: Vec<u32> = batches.iter()
            .flat_map(|b| {
                b.column(0)
                    .as_any()
                    .downcast_ref::<UInt32Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied()
            })
            .collect();
        
        uids.sort_unstable();
        Ok(uids)
    }
    
    /// Count messages
    pub async fn count(&self, mailbox_id: &[u8; 16]) -> Result<u32> {
        Ok(self.search(mailbox_id, &Search::all()).await?.len() as u32)
    }
    
    /// Max UID
    pub async fn max_uid(&self, mailbox_id: &[u8; 16]) -> Result<u32> {
        Ok(self.search(mailbox_id, &Search::all()).await?
            .into_iter()
            .max()
            .unwrap_or(0))
    }
    
    /// Execute query
    async fn execute(&self, columns: &[&str], filter: Expr) -> Result<Vec<RecordBatch>> {
        // LanceDB currently needs SQL string for filter
        // Convert Expr to string representation
        let filter_str = format!("{}", filter);
        
        let query = self.table.query()
            .select(lancedb::query::Select::Columns(
                columns.iter().map(|s| s.to_string()).collect()
            ))
            .filter(filter_str);
        
        query.execute()
            .await
            .map_err(|e| Error::Lance(e.to_string()))?
            .try_collect()
            .await
            .map_err(|e| Error::Lance(e.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Search Builder (DataFusion Expr)
// ─────────────────────────────────────────────────────────────────────────────

/// IMAP SEARCH criteria → DataFusion Expr
#[derive(Default, Clone)]
pub struct Search {
    pub from: Option<String>,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub flags: FlagSearch,
    pub size: SizeSearch,
}

#[derive(Default, Clone)]
pub struct FlagSearch {
    pub seen: Option<bool>,
    pub flagged: Option<bool>,
    pub answered: Option<bool>,
    pub deleted: Option<bool>,
    pub draft: Option<bool>,
}

#[derive(Default, Clone)]
pub struct SizeSearch {
    pub larger: Option<i64>,
    pub smaller: Option<i64>,
}

impl Search {
    pub fn all() -> Self {
        Self::default()
    }
    
    pub fn unseen() -> Self {
        Self {
            flags: FlagSearch { seen: Some(false), ..Default::default() },
            ..Default::default()
        }
    }
    
    pub fn from(addr: impl Into<String>) -> Self {
        Self { from: Some(addr.into()), ..Default::default() }
    }
    
    pub fn subject(s: impl Into<String>) -> Self {
        Self { subject: Some(s.into()), ..Default::default() }
    }
    
    pub fn since(ts: i64) -> Self {
        Self { since: Some(ts), ..Default::default() }
    }
    
    /// Convert to DataFusion Expr
    pub fn to_expr(&self, mailbox_id: &[u8; 16]) -> Expr {
        let mut expr = mailbox_eq(mailbox_id);
        
        // FROM
        if let Some(ref from) = self.from {
            expr = expr.and(col("from_addr").like(lit(format!("%{}%", from))));
        }
        
        // TO
        if let Some(ref to) = self.to {
            expr = expr.and(col("to_addrs").like(lit(format!("%{}%", to))));
        }
        
        // SUBJECT
        if let Some(ref subj) = self.subject {
            expr = expr.and(col("subject").like(lit(format!("%{}%", subj))));
        }
        
        // SINCE
        if let Some(ts) = self.since {
            expr = expr.and(col("internal_date").gt_eq(lit(ts)));
        }
        
        // BEFORE
        if let Some(ts) = self.before {
            expr = expr.and(col("internal_date").lt(lit(ts)));
        }
        
        // FLAGS
        if let Some(seen) = self.flags.seen {
            expr = expr.and(flag_filter("\\Seen", seen));
        }
        if let Some(flagged) = self.flags.flagged {
            expr = expr.and(flag_filter("\\Flagged", flagged));
        }
        if let Some(answered) = self.flags.answered {
            expr = expr.and(flag_filter("\\Answered", answered));
        }
        if let Some(deleted) = self.flags.deleted {
            expr = expr.and(flag_filter("\\Deleted", deleted));
        }
        if let Some(draft) = self.flags.draft {
            expr = expr.and(flag_filter("\\Draft", draft));
        }
        
        // SIZE
        if let Some(larger) = self.size.larger {
            expr = expr.and(col("size").gt(lit(larger)));
        }
        if let Some(smaller) = self.size.smaller {
            expr = expr.and(col("size").lt(lit(smaller)));
        }
        
        expr
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Expression Builders
// ─────────────────────────────────────────────────────────────────────────────

/// mailbox_id = X'...'
fn mailbox_eq(id: &[u8; 16]) -> Expr {
    col("mailbox_id").eq(lit(id.to_vec()))
}

/// Flag presence/absence filter
fn flag_filter(flag: &str, present: bool) -> Expr {
    // array_contains(flags, flag) or NOT array_contains(flags, flag)
    let contains = datafusion::functions_array::expr_fn::array_has(
        col("flags"),
        lit(flag),
    );
    if present { contains } else { contains.not() }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_search_expr() {
        let mailbox = [0u8; 16];
        let search = Search::from("alice").to_expr(&mailbox);
        let s = format!("{}", search);
        assert!(s.contains("from_addr"));
        assert!(s.contains("alice"));
    }
    
    #[test]
    fn test_search_unseen() {
        let mailbox = [0u8; 16];
        let search = Search::unseen().to_expr(&mailbox);
        let s = format!("{}", search);
        assert!(s.contains("Seen"));
    }
}

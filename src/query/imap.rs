//! IMAP Query Translation
//!
//! Converts IMAP SEARCH syntax to SearchCriteria.
//!
//! ## IMAP SEARCH Keys → SQL
//!
//! ```text
//! ALL           → (no filter)
//! ANSWERED      → array_contains(flags, '\Answered')
//! DELETED       → array_contains(flags, '\Deleted')
//! FLAGGED       → array_contains(flags, '\Flagged')
//! NEW           → NOT array_contains(flags, '\Seen') AND array_contains(flags, '\Recent')
//! OLD           → NOT array_contains(flags, '\Recent')
//! SEEN          → array_contains(flags, '\Seen')
//! UNSEEN        → NOT array_contains(flags, '\Seen')
//! DRAFT         → array_contains(flags, '\Draft')
//! 
//! BCC <string>  → array_to_string(bcc_addrs, ',') LIKE '%string%'
//! CC <string>   → array_to_string(cc_addrs, ',') LIKE '%string%'
//! FROM <string> → from_addr LIKE '%string%'
//! TO <string>   → array_to_string(to_addrs, ',') LIKE '%string%'
//! SUBJECT <str> → subject LIKE '%string%'
//! 
//! BEFORE <date> → internal_date < date
//! ON <date>     → internal_date >= date AND internal_date < date+1day
//! SINCE <date>  → internal_date >= date
//! SENTBEFORE    → date < ...
//! SENTON        → date = ...
//! SENTSINCE     → date >= ...
//! 
//! LARGER <n>    → size > n
//! SMALLER <n>   → size < n
//! 
//! UID <set>     → uid IN (...)
//! ```

use super::messages::SearchCriteria;

/// Parse IMAP SEARCH command into SearchCriteria
pub fn parse_search(tokens: &[&str]) -> SearchCriteria {
    let mut criteria = SearchCriteria::default();
    let mut i = 0;
    
    while i < tokens.len() {
        let token = tokens[i].to_uppercase();
        match token.as_str() {
            "ALL" => {
                // No filter
            }
            "UNSEEN" => {
                criteria.unseen = true;
            }
            "SEEN" => {
                criteria.unseen = false; // TODO: seen flag
            }
            "FLAGGED" => {
                criteria.flagged = true;
            }
            "ANSWERED" => {
                criteria.answered = true;
            }
            "DELETED" => {
                criteria.deleted = true;
            }
            "FROM" => {
                i += 1;
                if i < tokens.len() {
                    criteria.from = Some(tokens[i].to_string());
                }
            }
            "TO" => {
                i += 1;
                if i < tokens.len() {
                    criteria.to = Some(tokens[i].to_string());
                }
            }
            "SUBJECT" => {
                i += 1;
                if i < tokens.len() {
                    criteria.subject = Some(tokens[i].to_string());
                }
            }
            "SINCE" => {
                i += 1;
                if i < tokens.len() {
                    if let Some(ts) = parse_imap_date(tokens[i]) {
                        criteria.since = Some(ts);
                    }
                }
            }
            "BEFORE" => {
                i += 1;
                if i < tokens.len() {
                    if let Some(ts) = parse_imap_date(tokens[i]) {
                        criteria.before = Some(ts);
                    }
                }
            }
            _ => {
                // Unknown token, skip
            }
        }
        i += 1;
    }
    
    criteria
}

/// Parse IMAP date format (e.g., "1-Jan-2025") to milliseconds
fn parse_imap_date(s: &str) -> Option<i64> {
    // TODO: Proper IMAP date parsing
    // For now, just return None
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_search_unseen() {
        let criteria = parse_search(&["UNSEEN"]);
        assert!(criteria.unseen);
    }
    
    #[test]
    fn test_parse_search_from() {
        let criteria = parse_search(&["FROM", "alice@example.com"]);
        assert_eq!(criteria.from, Some("alice@example.com".to_string()));
    }
    
    #[test]
    fn test_parse_search_combined() {
        let criteria = parse_search(&["UNSEEN", "FROM", "bob", "SUBJECT", "hello"]);
        assert!(criteria.unseen);
        assert_eq!(criteria.from, Some("bob".to_string()));
        assert_eq!(criteria.subject, Some("hello".to_string()));
    }
}

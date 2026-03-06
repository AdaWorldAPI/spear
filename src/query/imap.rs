//! IMAP SEARCH Parser
//!
//! Parses IMAP SEARCH commands into Search expressions.
//!
//! ```text
//! IMAP                     Search
//! ──────────────────────────────────────────────
//! ALL                   →  Search::all()
//! UNSEEN                →  Search::unseen()
//! FROM "alice"          →  Search::from("alice")
//! SUBJECT "hello"       →  Search::subject("hello")
//! SINCE 1-Jan-2025      →  Search::since(timestamp)
//! UNSEEN FROM "bob"     →  Search::unseen().and_from("bob")
//! ```

use super::messages::{Search, FlagSearch, SizeSearch};

/// Parse IMAP SEARCH tokens into Search
pub fn parse(tokens: &[&str]) -> Search {
    let mut search = Search::default();
    let mut i = 0;
    
    while i < tokens.len() {
        match tokens[i].to_uppercase().as_str() {
            "ALL" => {}
            
            // Flags
            "UNSEEN" => search.flags.seen = Some(false),
            "SEEN" => search.flags.seen = Some(true),
            "FLAGGED" => search.flags.flagged = Some(true),
            "UNFLAGGED" => search.flags.flagged = Some(false),
            "ANSWERED" => search.flags.answered = Some(true),
            "UNANSWERED" => search.flags.answered = Some(false),
            "DELETED" => search.flags.deleted = Some(true),
            "UNDELETED" => search.flags.deleted = Some(false),
            "DRAFT" => search.flags.draft = Some(true),
            "UNDRAFT" => search.flags.draft = Some(false),
            
            // Headers with argument
            "FROM" => {
                i += 1;
                if i < tokens.len() {
                    search.from = Some(unquote(tokens[i]));
                }
            }
            "TO" => {
                i += 1;
                if i < tokens.len() {
                    search.to = Some(unquote(tokens[i]));
                }
            }
            "SUBJECT" => {
                i += 1;
                if i < tokens.len() {
                    search.subject = Some(unquote(tokens[i]));
                }
            }
            
            // Date
            "SINCE" => {
                i += 1;
                if i < tokens.len() {
                    if let Some(ts) = parse_date(tokens[i]) {
                        search.since = Some(ts);
                    }
                }
            }
            "BEFORE" => {
                i += 1;
                if i < tokens.len() {
                    if let Some(ts) = parse_date(tokens[i]) {
                        search.before = Some(ts);
                    }
                }
            }
            
            // Size
            "LARGER" => {
                i += 1;
                if i < tokens.len() {
                    if let Ok(n) = tokens[i].parse() {
                        search.size.larger = Some(n);
                    }
                }
            }
            "SMALLER" => {
                i += 1;
                if i < tokens.len() {
                    if let Ok(n) = tokens[i].parse() {
                        search.size.smaller = Some(n);
                    }
                }
            }
            
            _ => {} // Skip unknown
        }
        i += 1;
    }
    
    search
}

/// Remove surrounding quotes
fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || 
       (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len()-1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse IMAP date (e.g., "1-Jan-2025") to milliseconds since epoch
fn parse_date(s: &str) -> Option<i64> {
    // Format: DD-Mon-YYYY (e.g., "1-Jan-2025", "15-Mar-2024")
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    
    let day: u32 = parts[0].parse().ok()?;
    let month = match parts[1].to_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    let year: i32 = parts[2].parse().ok()?;
    
    // Convert to timestamp (milliseconds since epoch)
    // Simplified calculation (doesn't handle all edge cases)
    let days_since_epoch = days_from_ymd(year, month, day)?;
    Some(days_since_epoch * 24 * 60 * 60 * 1000)
}

/// Calculate days since Unix epoch for a date
fn days_from_ymd(year: i32, month: u32, day: u32) -> Option<i64> {
    // Simplified - use a proper date library in production
    if year < 1970 || month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }
    
    let mut days: i64 = 0;
    
    // Years since 1970
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    
    // Months in current year
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_month[(m - 1) as usize] as i64;
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }
    
    // Days
    days += (day - 1) as i64;
    
    Some(days)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_unseen() {
        let s = parse(&["UNSEEN"]);
        assert_eq!(s.flags.seen, Some(false));
    }
    
    #[test]
    fn test_parse_from() {
        let s = parse(&["FROM", "\"alice@example.com\""]);
        assert_eq!(s.from, Some("alice@example.com".to_string()));
    }
    
    #[test]
    fn test_parse_combined() {
        let s = parse(&["UNSEEN", "FROM", "bob", "SUBJECT", "hello"]);
        assert_eq!(s.flags.seen, Some(false));
        assert_eq!(s.from, Some("bob".to_string()));
        assert_eq!(s.subject, Some("hello".to_string()));
    }
    
    #[test]
    fn test_parse_date() {
        let ts = parse_date("1-Jan-2025").unwrap();
        assert!(ts > 0);
    }
    
    #[test]
    fn test_unquote() {
        assert_eq!(unquote("\"hello\""), "hello");
        assert_eq!(unquote("hello"), "hello");
    }
}

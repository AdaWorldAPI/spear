# Spear Integration Plan

## Phase 0: Foundation (Current)

### Completed
- [x] Repository structure
- [x] Schema definitions (messages, folders, accounts, events, contacts)
- [x] DataFusion expression builders
- [x] IMAP SEARCH parser
- [x] Content store (SHA256-addressed)
- [x] Architecture documentation

### Blockers
- [ ] Verify LanceDB 0.23 compilation
- [ ] Verify DataFusion 49 array functions
- [ ] Arrow 56 compatibility

---

## Phase 1: Core Storage

### 1.1 Write Operations
```rust
impl Spear {
    async fn create_account(&self, username: &str, email: &str) -> Result<[u8; 16]>;
    async fn create_folder(&self, account_id: &[u8; 16], path: &str) -> Result<[u8; 16]>;
    async fn append_message(&self, mailbox_id: &[u8; 16], msg: Message) -> Result<[u8; 16]>;
    async fn update_flags(&self, mailbox_id: &[u8; 16], uid: u32, flags: Vec<String>) -> Result<()>;
    async fn expunge(&self, mailbox_id: &[u8; 16]) -> Result<Vec<u32>>;
}
```

### 1.2 Read Operations
```rust
impl Spear {
    async fn get_folder(&self, account_id: &[u8; 16], path: &str) -> Result<Folder>;
    async fn list_folders(&self, account_id: &[u8; 16]) -> Result<Vec<Folder>>;
    async fn fetch_message(&self, mailbox_id: &[u8; 16], uid: u32, parts: FetchParts) -> Result<Message>;
    async fn search(&self, mailbox_id: &[u8; 16], criteria: Search) -> Result<Vec<u32>>;
}
```

### 1.3 Integration Test
```rust
#[tokio::test]
async fn test_mail_lifecycle() {
    let db = Spear::open(tempdir()).await?;
    
    // Create account
    let account = db.create_account("alice", "alice@example.com").await?;
    
    // Create INBOX
    let inbox = db.create_folder(&account, "INBOX").await?;
    
    // Append message
    let msg_id = db.append_message(&inbox, Message {
        from_addr: "bob@example.com".into(),
        subject: "Hello".into(),
        body: b"Test message".to_vec(),
        ..Default::default()
    }).await?;
    
    // Fetch FLAGS only (columnar!)
    let flags = db.fetch_message(&inbox, 1, FetchParts::FLAGS).await?;
    
    // Search
    let uids = db.search(&inbox, Search::from("bob")).await?;
    assert_eq!(uids, vec![1]);
}
```

### Deliverables
- [ ] `src/ops/write.rs` - Write operations
- [ ] `src/ops/read.rs` - Read operations
- [ ] `tests/integration.rs` - Full lifecycle test

---

## Phase 2: Protocol Layer

### 2.1 Stalwart Protocol Crates

Extract parsing only, no storage:

```toml
[dependencies]
imap-proto = { git = "https://github.com/stalwartlabs/mail-server", features = ["parse-only"] }
```

Or fork minimal versions:
- `spear-imap-proto` - IMAP command parsing
- `spear-jmap-proto` - JMAP request parsing

### 2.2 IMAP Handler

```rust
pub struct ImapSession {
    db: Arc<Spear>,
    account_id: [u8; 16],
    selected: Option<[u8; 16]>,  // Current mailbox
}

impl ImapSession {
    async fn handle(&mut self, cmd: ImapCommand) -> ImapResponse {
        match cmd {
            ImapCommand::Select(path) => self.select(&path).await,
            ImapCommand::Fetch(seq, parts) => self.fetch(seq, parts).await,
            ImapCommand::Search(criteria) => self.search(criteria).await,
            ImapCommand::Store(seq, flags) => self.store(seq, flags).await,
            ImapCommand::Expunge => self.expunge().await,
            // ...
        }
    }
}
```

### 2.3 JMAP Handler

```rust
pub struct JmapSession {
    db: Arc<Spear>,
    account_id: [u8; 16],
}

impl JmapSession {
    async fn handle(&mut self, req: JmapRequest) -> JmapResponse {
        match req.method {
            "Email/get" => self.email_get(req.args).await,
            "Email/query" => self.email_query(req.args).await,
            "Email/set" => self.email_set(req.args).await,
            "Mailbox/get" => self.mailbox_get(req.args).await,
            // ...
        }
    }
}
```

### Deliverables
- [ ] `src/protocol/imap.rs` - IMAP session handler
- [ ] `src/protocol/jmap.rs` - JMAP session handler
- [ ] `src/protocol/mod.rs` - Protocol dispatch

---

## Phase 3: Server

### 3.1 TCP/TLS Listener

```rust
#[tokio::main]
async fn main() {
    let db = Arc::new(Spear::open("./data").await?);
    
    // IMAP on 993 (TLS)
    let imap = TcpListener::bind("0.0.0.0:993").await?;
    
    // JMAP on 443 (HTTPS)
    let jmap = TcpListener::bind("0.0.0.0:443").await?;
    
    tokio::select! {
        _ = serve_imap(imap, db.clone()) => {},
        _ = serve_jmap(jmap, db.clone()) => {},
    }
}
```

### 3.2 Authentication

```rust
pub trait Authenticator {
    async fn authenticate(&self, username: &str, password: &str) -> Result<[u8; 16]>;
}

// Implementations
pub struct PlainAuth { db: Arc<Spear> }      // Password in accounts table
pub struct LdapAuth { ldap: LdapClient }     // External LDAP
pub struct OAuthAuth { provider: String }    // OAuth2/OIDC
```

### Deliverables
- [ ] `src/server/mod.rs` - Main server
- [ ] `src/server/imap.rs` - IMAP listener
- [ ] `src/server/jmap.rs` - JMAP listener
- [ ] `src/auth/mod.rs` - Authentication

---

## Phase 4: CalDAV/CardDAV

### 4.1 Calendar Operations

```rust
impl Spear {
    async fn create_calendar(&self, account_id: &[u8; 16], name: &str) -> Result<[u8; 16]>;
    async fn create_event(&self, calendar_id: &[u8; 16], event: Event) -> Result<[u8; 16]>;
    async fn get_events(&self, calendar_id: &[u8; 16], range: TimeRange) -> Result<Vec<Event>>;
    async fn update_event(&self, event_id: &[u8; 16], event: Event) -> Result<()>;
    async fn delete_event(&self, event_id: &[u8; 16]) -> Result<()>;
}
```

### 4.2 Contact Operations

```rust
impl Spear {
    async fn create_addressbook(&self, account_id: &[u8; 16], name: &str) -> Result<[u8; 16]>;
    async fn create_contact(&self, book_id: &[u8; 16], contact: Contact) -> Result<[u8; 16]>;
    async fn search_contacts(&self, book_id: &[u8; 16], query: &str) -> Result<Vec<Contact>>;
    async fn update_contact(&self, contact_id: &[u8; 16], contact: Contact) -> Result<()>;
    async fn delete_contact(&self, contact_id: &[u8; 16]) -> Result<()>;
}
```

### Deliverables
- [ ] `src/ops/calendar.rs` - Calendar operations
- [ ] `src/ops/contacts.rs` - Contact operations
- [ ] `src/protocol/caldav.rs` - CalDAV handler
- [ ] `src/protocol/carddav.rs` - CardDAV handler

---

## Phase 5: Migration Tools

### 5.1 IMAP Import

```rust
pub struct ImapMigrator {
    source: ImapClient,
    target: Arc<Spear>,
}

impl ImapMigrator {
    async fn migrate_account(&self, account_id: &[u8; 16]) -> Result<MigrationStats> {
        // List folders
        // For each folder:
        //   - Create in Spear
        //   - Fetch all messages
        //   - Append to Spear (columnar!)
    }
}
```

### 5.2 mbox Import

```rust
pub fn import_mbox(path: &Path, db: &Spear, mailbox_id: &[u8; 16]) -> Result<u32>;
```

### 5.3 Maildir Import

```rust
pub fn import_maildir(path: &Path, db: &Spear, account_id: &[u8; 16]) -> Result<Stats>;
```

### Deliverables
- [ ] `src/migrate/imap.rs` - IMAP migration
- [ ] `src/migrate/mbox.rs` - mbox import
- [ ] `src/migrate/maildir.rs` - Maildir import
- [ ] `spear-migrate` binary

---

## Phase 6: Production Hardening

### 6.1 Replication

```rust
// Leader-follower replication via Lance versioning
pub struct ReplicationManager {
    leader: Spear,
    followers: Vec<Spear>,
}
```

### 6.2 Backup

```rust
// Lance snapshots + content store rsync
pub async fn backup(db: &Spear, dest: &Path) -> Result<()>;
pub async fn restore(src: &Path, db: &Spear) -> Result<()>;
```

### 6.3 Monitoring

```rust
// Prometheus metrics
pub struct Metrics {
    messages_total: Counter,
    fetch_latency: Histogram,
    search_latency: Histogram,
    storage_bytes: Gauge,
}
```

### Deliverables
- [ ] `src/replication/mod.rs` - Replication
- [ ] `src/backup/mod.rs` - Backup/restore
- [ ] `src/metrics/mod.rs` - Prometheus metrics
- [ ] Grafana dashboards

---

## Timeline

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| 0: Foundation | Done | - |
| 1: Core Storage | 1 week | Phase 0 |
| 2: Protocol Layer | 2 weeks | Phase 1 |
| 3: Server | 1 week | Phase 2 |
| 4: CalDAV/CardDAV | 1 week | Phase 3 |
| 5: Migration | 1 week | Phase 3 |
| 6: Production | 2 weeks | Phase 5 |

**Total: ~8 weeks to production-ready**

---

## Success Criteria

### Performance
- FETCH FLAGS: < 1ms for 10K messages (columnar scan)
- SEARCH: < 100ms for 100K messages (predicate pushdown)
- APPEND: < 10ms per message

### Compression
- 10x compression vs raw JSON
- 3x compression vs Dovecot index

### Compatibility
- Pass IMAP compliance tests
- Pass JMAP compliance tests
- Work with Thunderbird, Apple Mail, Outlook

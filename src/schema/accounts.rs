//! Accounts Schema
//!
//! User accounts with authentication and quota.

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use std::sync::LazyLock;

pub static ACCOUNTS_SCHEMA: LazyLock<Schema> = LazyLock::new(schema);

pub fn schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(16), false),
        Field::new("username", DataType::Utf8, false),
        Field::new("password_hash", DataType::Binary, false),
        Field::new("email", DataType::Utf8, false),
        Field::new("display_name", DataType::Utf8, true),
        Field::new("quota_bytes", DataType::Int64, false),
        Field::new("used_bytes", DataType::Int64, false),
        Field::new("created_at", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("last_login", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), true),
        Field::new("active", DataType::Boolean, false),
    ])
}

pub mod col {
    pub const ID: usize = 0;
    pub const USERNAME: usize = 1;
    pub const PASSWORD_HASH: usize = 2;
    pub const EMAIL: usize = 3;
    pub const DISPLAY_NAME: usize = 4;
    pub const QUOTA_BYTES: usize = 5;
    pub const USED_BYTES: usize = 6;
    pub const CREATED_AT: usize = 7;
    pub const LAST_LOGIN: usize = 8;
    pub const ACTIVE: usize = 9;
}

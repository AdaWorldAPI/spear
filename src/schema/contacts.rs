//! Contacts Schema (CardDAV)
//!
//! Address book contacts.

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use std::sync::{Arc, LazyLock};

pub static CONTACTS_SCHEMA: LazyLock<Schema> = LazyLock::new(schema);

pub fn schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(16), false),
        Field::new("account_id", DataType::FixedSizeBinary(16), false),
        Field::new("addressbook_id", DataType::FixedSizeBinary(16), false),
        Field::new("uid", DataType::Utf8, false),
        Field::new("display_name", DataType::Utf8, true),
        Field::new("first_name", DataType::Utf8, true),
        Field::new("last_name", DataType::Utf8, true),
        Field::new("nickname", DataType::Utf8, true),
        Field::new("emails", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("phones", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("addresses", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("organization", DataType::Utf8, true),
        Field::new("title", DataType::Utf8, true),
        Field::new("notes", DataType::Utf8, true),
        Field::new("photo_ref", DataType::Utf8, true),
        Field::new("birthday", DataType::Date32, true),
        Field::new("categories", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("created_at", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("modified_at", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("etag", DataType::Utf8, false),
    ])
}

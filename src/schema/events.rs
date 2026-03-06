//! Events Schema (CalDAV)
//!
//! Calendar events with recurrence support.

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use std::sync::{Arc, LazyLock};

pub static EVENTS_SCHEMA: LazyLock<Schema> = LazyLock::new(schema);

pub fn schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(16), false),
        Field::new("account_id", DataType::FixedSizeBinary(16), false),
        Field::new("calendar_id", DataType::FixedSizeBinary(16), false),
        Field::new("uid", DataType::Utf8, false),
        Field::new("summary", DataType::Utf8, true),
        Field::new("description", DataType::Utf8, true),
        Field::new("location", DataType::Utf8, true),
        Field::new("start_time", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("end_time", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("all_day", DataType::Boolean, false),
        Field::new("timezone", DataType::Utf8, true),
        Field::new("recurrence_rule", DataType::Utf8, true),
        Field::new("recurrence_id", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), true),
        Field::new("organizer", DataType::Utf8, true),
        Field::new("attendees", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("status", DataType::Utf8, true),
        Field::new("transparency", DataType::Utf8, true),
        Field::new("categories", DataType::List(Arc::new(
            Field::new("item", DataType::Utf8, false)
        )), true),
        Field::new("created_at", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("modified_at", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false),
        Field::new("etag", DataType::Utf8, false),
    ])
}

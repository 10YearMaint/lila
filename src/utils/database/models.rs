use crate::schema::{file_content, metadata};
use diesel::prelude::*;
use diesel::Queryable;

/// Represents a row in the `metadata` table
#[derive(Queryable, Insertable)]
#[diesel(table_name = metadata)]
pub struct Metadata {
    pub id: i32,
    pub file_path: String,
}

/// Represents a row in the `file_content` table
#[derive(Queryable, Insertable)]
#[diesel(table_name = file_content)]
pub struct FileContent {
    // Same primary key as `metadata.id`
    pub id: i32,
    pub content: String,
}

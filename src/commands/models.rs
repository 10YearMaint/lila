use crate::schema::{html_content, html_metadata};
use diesel::prelude::*;
use diesel::Queryable;

/// Represents a row in the `html_metadata` table
#[derive(Queryable, Insertable)]
#[diesel(table_name = html_metadata)]
pub struct HtmlMetadata {
    pub id: i32,
    pub file_path: String,
}

/// Represents a row in the `html_content` table
#[derive(Queryable, Insertable)]
#[diesel(table_name = html_content)]
pub struct HtmlContent {
    // Same primary key as `html_metadata.id`
    pub id: i32,
    pub content: String,
}

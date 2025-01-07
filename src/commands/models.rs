// src/commands/models.rs
use crate::schema::html_metadata;
use diesel::prelude::*;

#[derive(Queryable, Insertable)]
#[diesel(table_name = html_metadata)]
pub struct HtmlMetadata {
    pub id: Option<i32>,
    pub file_path: String,
}

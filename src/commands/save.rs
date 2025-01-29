use crate::schema::{html_content, html_metadata};
use crate::utils::database::models::{HtmlContent, HtmlMetadata};
use colored::Colorize;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Text};
use diesel::sqlite::SqliteConnection;
use dotenvy::dotenv;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Small struct for checking if a table exists.
#[derive(QueryableByName)]
struct Exists {
    #[diesel(sql_type = Text)]
    #[allow(dead_code)]
    name: String,
}

/// To fetch the SQLite `last_insert_rowid()` result.
#[derive(QueryableByName)]
struct LastInsertRowId {
    #[diesel(sql_type = BigInt)]
    last_insert_rowid: i64,
}

/// Establish a DB connection using the `DATABASE_URL` env variable.
pub fn establish_connection(database_url: &str) -> SqliteConnection {
    dotenv().ok();
    SqliteConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {database_url}"))
}

/// Check if a given table exists in SQLite.
fn table_exists(conn: &mut SqliteConnection, table_name: &str) -> bool {
    let query =
        format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table_name}';");
    let result: Result<Option<Exists>, _> = sql_query(query).get_result(conn);
    result.map(|res| res.is_some()).unwrap_or(false)
}

/// Run Diesel migrations. Panics if migrations fail.
fn run_migrations(database_url: &str) {
    let output = Command::new("diesel")
        .arg("migration")
        .arg("run")
        .env("DATABASE_URL", database_url)
        .output()
        .expect("Failed to execute migration command");

    if !output.status.success() {
        panic!(
            "Migration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Insert or update HTML files' metadata/content in the DB.
pub fn save_html_metadata_to_db(
    html_files: &[String],
    conn: &mut SqliteConnection,
    database_url: &str,
) -> Result<(), Error> {
    // Alias each DSL to avoid `id` name collisions:
    use html_content::dsl as ct;
    use html_metadata::dsl as md;

    // 1) Check if tables exist; if not, run migrations and re-connect.
    if !table_exists(conn, "html_metadata") || !table_exists(conn, "html_content") {
        tracing::info!(
            "Tables 'html_metadata' or 'html_content' do not exist. Running migrations..."
        );
        run_migrations(database_url);
        *conn = establish_connection(database_url);
    }

    // 2) Use a Diesel transaction for multiple inserts/updates.
    conn.transaction::<(), Error, _>(|c| {
        for path_str in html_files {
            // Attempt to read file content
            let path_obj = Path::new(path_str);
            let content_val = fs::read_to_string(path_obj).unwrap_or_else(|_| {
                "<html><body><p>Failed to read HTML content.</p></body></html>".to_string()
            });

            // Check if we already have a row in `html_metadata` for this file_path
            let existing = md::html_metadata
                .filter(md::file_path.eq(path_str))
                .first::<HtmlMetadata>(c);

            match existing {
                Ok(record) => {
                    // UPDATE CASE: The record already exists in `html_metadata`.
                    // Update the `content` in `html_content` referencing the same id:
                    diesel::update(ct::html_content.find(record.id))
                        .set(ct::content.eq(content_val))
                        .execute(c)?;

                    tracing::info!("Updated content for file_path '{}'", path_str);
                }
                Err(diesel::result::Error::NotFound) => {
                    // INSERT CASE: No row for this path -> Insert into `html_metadata` first.
                    diesel::insert_into(md::html_metadata)
                        .values(md::file_path.eq(path_str))
                        .execute(c)?;

                    // Then fetch the new `id` from SQLite's last_insert_rowid().
                    let row: LastInsertRowId =
                        diesel::sql_query("SELECT last_insert_rowid() as last_insert_rowid")
                            .get_result(c)?;

                    let new_id = row.last_insert_rowid as i32;

                    // Insert into `html_content` with the same `id`.
                    diesel::insert_into(ct::html_content)
                        .values((ct::id.eq(new_id), ct::content.eq(content_val)))
                        .execute(c)?;

                    tracing::info!("Inserted metadata and content for '{}'", path_str);
                }
                Err(e) => {
                    // Some other database error
                    tracing::error!("Error querying metadata for '{}': {:?}", path_str, e);
                    return Err(e);
                }
            }
        }

        println!(
            "{}",
            "Successfully saved HTML metadata and content to the database.".green()
        );
        Ok(())
    })
}

use crate::schema::{file_content, metadata};
use crate::utils::database::models::Metadata;
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
        .expect("Failed to run migrations via Diesel CLI");

    if !output.status.success() {
        panic!(
            "Migration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Generic function to insert or update any text files in the DB
/// (whether they're HTML or Markdown).
pub fn save_files_to_db(
    file_paths: &[String],
    conn: &mut SqliteConnection,
    database_url: &str,
) -> Result<(), Error> {
    // Bring in the DSL so we have access to the table and columns
    use file_content::dsl as c;
    use metadata::dsl as m;

    // 1) Ensure the `metadata` and `file_content` tables exist
    if !table_exists(conn, "metadata") || !table_exists(conn, "file_content") {
        tracing::info!("Tables 'metadata' or 'file_content' do not exist. Running migrations...");
        run_migrations(database_url);
        *conn = establish_connection(database_url);
    }

    // 2) Use a transaction to insert/update all files at once
    conn.transaction::<(), Error, _>(|trx_conn| {
        for path_str in file_paths {
            let path_obj = Path::new(path_str);
            let file_data = fs::read_to_string(path_obj)
                .unwrap_or_else(|_| "<empty or unreadable>".to_string());

            // Check if there's already a row in `metadata` for this file_path
            let existing = m::metadata
                .filter(m::file_path.eq(path_str))
                .first::<Metadata>(trx_conn);

            match existing {
                Ok(record) => {
                    // Record already exists -> update the file_content table
                    diesel::update(c::file_content.find(record.id))
                        .set(c::content.eq(file_data))
                        .execute(trx_conn)?;

                    tracing::info!("Updated content for {}", path_str);
                }
                Err(diesel::result::Error::NotFound) => {
                    // Insert new metadata row first
                    diesel::insert_into(m::metadata)
                        .values(m::file_path.eq(path_str))
                        .execute(trx_conn)?;

                    // Then fetch that new row's `id`
                    let row: LastInsertRowId =
                        sql_query("SELECT last_insert_rowid() as last_insert_rowid")
                            .get_result(trx_conn)?;

                    // Insert content using that same `id`
                    diesel::insert_into(c::file_content)
                        .values((
                            c::id.eq(row.last_insert_rowid as i32),
                            c::content.eq(file_data),
                        ))
                        .execute(trx_conn)?;

                    tracing::info!("Inserted metadata + content for {}", path_str);
                }
                Err(e) => {
                    tracing::error!("Error looking up metadata for '{}': {:?}", path_str, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    })?;

    println!("{}", "All files saved successfully!".green());
    Ok(())
}

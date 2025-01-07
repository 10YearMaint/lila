use crate::commands::models::HtmlMetadata;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sql_query;
use diesel::sql_types::Text;
use diesel::sqlite::SqliteConnection;
use dotenvy::dotenv;
use std::process::Command;

pub fn establish_connection(database_url: &str) -> SqliteConnection {
    dotenv().ok();
    SqliteConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

#[derive(QueryableByName)]
struct Exists {
    #[diesel(sql_type = Text)]
    #[allow(dead_code)]
    name: String,
}

fn table_exists(conn: &mut SqliteConnection, table_name: &str) -> bool {
    let query = format!(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='{}';",
        table_name
    );
    let result: Result<Option<Exists>, _> = sql_query(query).get_result(conn);
    result.map(|res| res.is_some()).unwrap_or(false)
}

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

pub fn save_html_metadata_to_db(
    html_files: &[String],
    conn: &mut SqliteConnection,
    database_url: &str,
) -> Result<(), Error> {
    use crate::schema::html_metadata::dsl::*;

    if !table_exists(conn, "html_metadata") {
        println!("Table 'html_metadata' does not exist. Running migrations...");
        run_migrations(database_url);
        // Reconnect to the database to refresh the schema
        *conn = establish_connection(database_url);
    }

    for path in html_files {
        let new_metadata = HtmlMetadata {
            id: None,
            file_path: path.clone(),
        };

        diesel::insert_into(html_metadata)
            .values(&new_metadata)
            .execute(conn)?; // Use mutable reference
    }

    println!("Saved HTML metadata to database");
    Ok(())
}

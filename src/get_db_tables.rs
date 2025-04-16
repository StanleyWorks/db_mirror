use sqlx::query;
use sqlx::Row;

use crate::{config::DbConfig, create_connection::create_connection};

/// Get all tables from the primary database.
/// These will be mirrored on the secondary DB
pub async fn get_db_tables(
    db_config: &DbConfig,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let schema = db_config.schema.clone();

    println!("Connecting to primary DB on: {:?}", db_config);
    let conn_one = create_connection(db_config).await?;

    println!("Connected to primary DB");

    let all_tables = query("SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = ?")
        .bind(schema)
        .bind("BASE TABLE")
        .fetch_all(&conn_one).await?;

    println!("Fetched all tables");
    let table_names: Vec<String> = all_tables
        .iter()
        .map(|row| {
            let raw_bytes: Vec<u8> = row.get("TABLE_NAME");
            String::from_utf8(raw_bytes).expect("Invalid UTF-8 in table name")
        })
        .collect();

    Ok(table_names)
}

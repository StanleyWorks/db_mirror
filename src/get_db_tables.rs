use log::info;
use sqlx::Error;
use sqlx::Row;
use sqlx::query;

use crate::{config::DbConfig, create_connection::create_connection};

/// Get all tables from the primary database.
/// These will be mirrored on the secondary DB
pub async fn get_db_tables(db_config: &DbConfig) -> Result<Vec<String>, Error> {
    info!(
        "Connecting to primary DB on {}:{} schema `{}` as `{}`",
        db_config.host, db_config.port, db_config.schema, db_config.user
    );
    let mut conn_one = create_connection(db_config).await?;

    info!("Connected to primary DB");

    let all_tables = query("SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = ?")
        .bind(&db_config.schema)
        .bind("BASE TABLE")
        .fetch_all(&mut conn_one).await?;

    info!("Fetched all tables");
    let table_names: Vec<String> = all_tables
        .into_iter()
        .map(|row| row.get("TABLE_NAME"))
        .collect();

    Ok(table_names)
}

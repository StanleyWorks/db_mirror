use sqlx::Row;
use std::{error::Error, fs};

use sqlx::query;

use crate::{config::Config, create_connection::create_connection, get_db_tables::get_db_tables};

/// Copy table data from the primary to secondary DB
pub async fn copy_tables() -> Result<(), Box<dyn Error>> {
    let config_str = fs::read_to_string("config.toml")?;
    let db_config: Config = toml::from_str(&config_str)?;

    let primary_db_tables = get_db_tables(&db_config.primary_db).await?;

    let primary_db = db_config.primary_db;
    let secondary_db = db_config.secondary_db;
    let secondary_db_connection = create_connection(&secondary_db).await?;
    let primary_db_connection = create_connection(&primary_db).await?;

    for table in primary_db_tables.iter() {
        println!("Checking if table {} exists in secondary", table);
        let count = query("SELECT COUNT(*) AS count FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?")
            .bind(&secondary_db.schema)
            .bind(table)
            .fetch_one(&secondary_db_connection).await?;

        if count.get::<i32, _>("count") != 0 {
            println!("Truncating {}", table);
            query("SET FOREIGN_KEY_CHECKS = 0")
                .execute(&secondary_db_connection)
                .await
                .map_err(|_| "Failed to set FOREIGN_KEY_CHECKS to 0")?;

            query(&format!(
                "TRUNCATE TABLE {}.{}",
                &secondary_db.schema, table
            ))
            .execute(&secondary_db_connection)
            .await
            .map_err(|e| format!("Failed to truncate {:?}", e))?;

            println!("Mirroring the {} table", table);
            let rows = query(&format!("SELECT * FROM `{}`", table))
                .fetch_all(&primary_db_connection)
                .await
                .map_err(|e| format!("Failed to fetch data from primary DB: {:?}", e))?;

            if !rows.is_empty() {
                let column_query = format!(
                    "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = '{}'",
                    table
                );

                let columns: Vec<String> = sqlx::query(&column_query)
                    .fetch_all(&secondary_db_connection)
                    .await?
                    .into_iter()
                    .map(|row| row.get::<String, _>("COLUMN_NAME"))
                    .collect();

                if columns.is_empty() {
                    return Err(format!("No columns found for table {}", table).into());
                }

                let num_columns = columns.len();
                let placeholders = vec!["?"; num_columns].join(", ");

                let insert_query = format!(
                    "INSERT INTO `{}` ({}) VALUES ({})",
                    table,
                    columns.join(", "), // Explicit column names
                    placeholders
                );

                println!("Executing: {}", insert_query);

                let mut transaction = secondary_db_connection.begin().await?;

                for row in rows {
                    let mut query_builder = sqlx::query(&insert_query);
                    for col in &columns {
                        let value: Option<String> = row.try_get(col.as_str()).ok();
                        query_builder = query_builder.bind(value);
                    }
                    let _ = query_builder.execute(&mut *transaction).await;
                }

                let _ = transaction.commit().await;
            }
        }
    }

    query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&secondary_db_connection)
        .await?;
    Ok(())
}

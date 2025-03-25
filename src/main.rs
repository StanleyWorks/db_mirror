use std::{error::Error, fs, time::Instant};

use serde::Deserialize;
use sqlx::{query, MySql, MySqlPool, Pool, Row};

#[derive(Debug, Deserialize)]
struct Config {
    primary_db: DbConfig,
    secondary_db: DbConfig,
}

#[derive(Debug, Deserialize)]
struct DbConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
    schema: String,
}

async fn create_connection(db_config: &DbConfig) -> Result<Pool<MySql>, Box<dyn Error>> {
    let connection_string = format!(
        "mysql://{}:{}@{}:{}/{}",
        db_config.user, db_config.password, db_config.host, db_config.port, db_config.schema
    );
    let pool = MySqlPool::connect(&connection_string)
        .await
        .map_err(|err| format!("Database connections error {:?}", err))?;

    Ok(pool)
}

/// Get all Databse tables
async fn get_db_tables(db_config: &DbConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let schema = db_config.schema.clone();

    println!("Connecting to primary DB on: {}", db_config.host);
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

async fn copy_tables() -> Result<(), Box<dyn Error>> {
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

#[tokio::main]
async fn main() {
    let start_time = Instant::now();

    match copy_tables().await {
        Ok(_) => {
            let elapsed = start_time.elapsed();
            println!("Database mirror completed in {:.2?}", elapsed)
        }
        Err(err) => eprintln!("{}", err),
    };
}

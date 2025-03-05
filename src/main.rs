use std::{error::Error, fs};

use mysql::{params, prelude::Queryable, OptsBuilder, Pool, Row};
use serde::Deserialize;

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

fn create_connection(db_config: &DbConfig) -> Result<Pool, mysql::Error> {
    let opts = OptsBuilder::new()
        .user(Some(db_config.user.clone()))
        .pass(Some(db_config.password.clone()))
        .ip_or_hostname(Some(db_config.host.clone()))
        .tcp_port(db_config.port);

    Pool::new(opts)
}

fn get_db_tables(db_config: &DbConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let schema = db_config.schema.clone();

    println!("Connecting to primary DB on: {}", db_config.host);
    let conn_one = create_connection(db_config)?;

    let mut tx1 = conn_one.get_conn()?;

    println!("Connected to primary DB");

    let stmt = tx1.prep(
        "SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = :primary_schema AND TABLE_TYPE = :table_type",
    )?;

    println!("Fetching tables");

    let all_tables = tx1.exec(
        &stmt,
        params! {"primary_schema" => schema, "table_type" => "BASE TABLE"},
    )?;

    Ok(all_tables)
}

fn copy_tables() -> Result<(), Box<dyn Error>> {
    let config_str = fs::read_to_string("config.toml")?;
    let db_config: Config = toml::from_str(&config_str)?;

    let primary_db_tables = get_db_tables(&db_config.primary_db)?;

    let primary_db = db_config.primary_db;
    let secondary_db = db_config.secondary_db;
    let secondary_schema = secondary_db.schema.clone();
    let secondary_db_connection = create_connection(&secondary_db)?;
    let primary_db_connection = create_connection(&primary_db)?;
    let mut pri_con = primary_db_connection.get_conn()?;
    let mut sec_con = secondary_db_connection.get_conn()?;
    let _: Vec<String> = sec_con.query("SET FOREIGN_KEY_CHECKS = 0")?;

    let _ = pri_con.query::<String, String>(format!("USE {}", primary_db.schema));

    for table in primary_db_tables.iter() {
        println!("Checking if table {} exists in secondary", table);
        let query = "SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?";
        let count: Option<u64> = sec_con.exec_first(query, (secondary_schema.clone(), table))?; // The clone here 😬

        if count.unwrap() != 0 {
            println!("Truncating {}", table);
            sec_con.query::<String, String>(format!(
                "TRUNCATE TABLE {}.{}",
                secondary_db.schema, table
            ))?;

            println!("Mirroring data");
            let rows: Vec<Row> = pri_con.query(format!("SELECT * FROM {}", table))?;

            if !rows.is_empty() {
                let num_columns = rows[0].columns().len();
                let placeholders = vec!["?"; num_columns].join(", ");
                let insert_query = format!("INSERT INTO `{}` VALUES ({})", table, placeholders);

                let mut values = Vec::new();
                for row in rows {
                    values.push(row.unwrap());
                }

                let _ = sec_con.exec_batch(insert_query, values);
            }
        }
    }

    let _: Vec<String> = sec_con.query("SET FOREIGN_KEY_CHECKS = 1")?;
    Ok(())
}

fn main() {
    match copy_tables() {
        Ok(_) => println!("Database mirror complete."),
        Err(err) => eprintln!("Error {}", err),
    };
}

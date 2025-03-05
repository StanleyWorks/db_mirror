use std::fs;

use mysql::{params, prelude::Queryable, OptsBuilder, Pool};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    primary_db: DbConfig,
}

#[derive(Debug, Deserialize)]
struct DbConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
    schema: String,
}

fn create_connection_one(db_config: DbConfig) -> Result<Pool, mysql::Error> {
    let opts = OptsBuilder::new()
        .user(Some(db_config.user))
        .pass(Some(db_config.password))
        .ip_or_hostname(Some(db_config.host))
        .tcp_port(db_config.port);

    Pool::new(opts)
}

fn get_primary_db_tables(db_config: DbConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let schema = db_config.schema.clone();
    let conn_one = create_connection_one(db_config)?;

    let mut tx1 = conn_one.get_conn()?;

    let stmt = tx1.prep(
        "SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = :primary_schema AND TABLE_TYPE = :table_type",
    )?;

    let all_tables = tx1.exec(
        &stmt,
        params! {"primary_schema" => schema, "table_type" => "BASE TABLE"},
    )?;

    Ok(all_tables)
}

fn main() {
    let config_str = fs::read_to_string("config.toml").expect("Cannot find the file");
    let db_one_config: Config = toml::from_str(&config_str).expect("No config.toml");

    let primary_db_tables = match get_primary_db_tables(db_one_config.primary_db) {
        Ok(tables) => tables,
        Err(err) => {
            eprint!("{}", err);
            panic!("Something went wrong with Primary DB.");
        }
    };

    for table in primary_db_tables.iter() {
        println!("{}", table)
    }
    println!("Hello, world!");
}

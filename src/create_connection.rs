use std::error::Error;

use sqlx::{MySql, MySqlPool, Pool};

use crate::config::DbConfig;

/// Create a database connection
/// It takes a db config object.
pub async fn create_connection(db_config: &DbConfig) -> Result<Pool<MySql>, Box<dyn Error>> {
    let connection_string = format!(
        "mysql://{}:{}@{}:{}/{}",
        db_config.user, db_config.password, db_config.host, db_config.port, db_config.schema
    );
    let pool = MySqlPool::connect(&connection_string)
        .await
        .map_err(|err| format!("Database connections error {:?}", err))?;

    Ok(pool)
}

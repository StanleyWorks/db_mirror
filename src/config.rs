use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub primary_db: DbConfig,
    pub secondary_db: DbConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub schema: String,
}

use serde::Deserialize;
use std::fmt;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub primary_db: DbConfig,
    pub secondary_db: DbConfig,
}

#[derive(Deserialize)]
pub(crate) struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub schema: String,
}

impl fmt::Debug for DbConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &"[redacted]")
            .field("schema", &self.schema)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::DbConfig;

    #[test]
    fn debug_redacts_password() {
        let config = DbConfig {
            host: "localhost".into(),
            port: 3306,
            user: "root".into(),
            password: "super-secret".into(),
            schema: "mirror".into(),
        };

        let rendered = format!("{config:?}");

        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("super-secret"));
    }
}

use sqlx::{Connection, Error, MySqlConnection, mysql::MySqlConnectOptions};

use crate::config::DbConfig;

pub(crate) fn build_connect_options(db_config: &DbConfig) -> MySqlConnectOptions {
    MySqlConnectOptions::new()
        .host(&db_config.host)
        .port(db_config.port)
        .username(&db_config.user)
        .password(&db_config.password)
        .database(&db_config.schema)
}

/// Create a database connection using structured options so credentials
/// do not need to be URL-encoded manually.
pub async fn create_connection(db_config: &DbConfig) -> Result<MySqlConnection, Error> {
    let options = build_connect_options(db_config);
    MySqlConnection::connect_with(&options).await
}

#[cfg(test)]
mod tests {
    use super::build_connect_options;
    use crate::config::DbConfig;

    #[test]
    fn build_connect_options_preserves_fields() {
        let config = DbConfig {
            host: "db.internal".into(),
            port: 3307,
            user: "mirror-user".into(),
            password: "p@ss:w/or?d#1".into(),
            schema: "reporting".into(),
        };

        let options = build_connect_options(&config);

        assert_eq!(options.get_host(), "db.internal");
        assert_eq!(options.get_port(), 3307);
        assert_eq!(options.get_username(), "mirror-user");
        assert_eq!(options.get_database(), Some("reporting"));
    }
}

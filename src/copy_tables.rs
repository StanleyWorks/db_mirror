use log::{info, warn};
use sqlx::{Connection, Error, MySqlConnection, Row};
use std::fs;

use sqlx::query;

use crate::{config::Config, create_connection::create_connection, get_db_tables::get_db_tables};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnSpec {
    name: String,
    data_type: String,
}

/// Copy table data from the primary to secondary DB
pub async fn copy_tables() -> Result<(), Error> {
    let config_str = fs::read_to_string("config.toml")?;
    let db_config: Config = toml::from_str(&config_str)
        .map_err(|err| Error::protocol(format!("Database config loading failed: {err}")))?;

    let primary_db_tables = get_db_tables(&db_config.primary_db).await?;

    let primary_db = db_config.primary_db;
    let secondary_db = db_config.secondary_db;
    let mut secondary_db_connection = create_connection(&secondary_db).await?;
    let mut primary_db_connection = create_connection(&primary_db).await?;

    query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(&mut secondary_db_connection)
        .await?;

    let mirror_result = async {
        for table in &primary_db_tables {
            mirror_table(
                table,
                &primary_db.schema,
                &secondary_db.schema,
                &mut primary_db_connection,
                &mut secondary_db_connection,
            )
            .await?;
        }
        Ok(())
    }
    .await;

    let restore_result = query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&mut secondary_db_connection)
        .await;

    match (mirror_result, restore_result) {
        (Err(err), Ok(_)) => {
            warn!("Database mirror failed: {err}");
            Err(err)
        }
        (Err(err), Err(restore_err)) => {
            warn!("Database mirror failed: {err}");
            warn!("Failed to restore foreign key checks: {restore_err}");
            Err(err)
        }
        (Ok(_), Err(err)) => Err(err),
        (Ok(_), Ok(_)) => Ok(()),
    }
}

async fn mirror_table(
    table: &str,
    primary_schema: &str,
    secondary_schema: &str,
    primary_conn: &mut MySqlConnection,
    secondary_conn: &mut MySqlConnection,
) -> Result<(), Error> {
    info!("Checking if table {table} exists in secondary");

    if !secondary_table_exists(secondary_conn, secondary_schema, table).await? {
        warn!("Skipping {table}: table does not exist in secondary schema");
        return Ok(());
    }

    let columns = get_table_columns(primary_conn, primary_schema, table).await?;
    if columns.is_empty() {
        return Err(Error::RowNotFound);
    }

    info!("Truncating {table}");
    query(&format!(
        "TRUNCATE TABLE {}",
        qualified_table_name(secondary_schema, table)
    ))
    .execute(&mut *secondary_conn)
    .await?;

    let select_query = build_literal_select_query(primary_schema, table, &columns)?;
    let rows = query(&select_query).fetch_all(&mut *primary_conn).await?;

    info!("Mirroring the {table} table");
    if rows.is_empty() {
        info!("Found 0 rows.");
        return Ok(());
    }

    info!("Found {} rows.", rows.len());

    let mut transaction = secondary_conn.begin().await?;
    for row in rows {
        let insert_query = build_insert_query(secondary_schema, table, &columns, &row)?;
        query(&insert_query).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;

    Ok(())
}

async fn secondary_table_exists(
    secondary_conn: &mut MySqlConnection,
    secondary_schema: &str,
    table: &str,
) -> Result<bool, Error> {
    let count = query(
        "SELECT COUNT(*) AS count FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?",
    )
    .bind(secondary_schema)
    .bind(table)
    .fetch_one(&mut *secondary_conn)
    .await?;

    Ok(count.get::<i64, _>("count") != 0)
}

async fn get_table_columns(
    conn: &mut MySqlConnection,
    schema: &str,
    table: &str,
) -> Result<Vec<ColumnSpec>, Error> {
    query(
        "SELECT COLUMN_NAME, DATA_TYPE \
         FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
         ORDER BY ORDINAL_POSITION",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        Ok(ColumnSpec {
            name: row.try_get("COLUMN_NAME")?,
            data_type: row.try_get::<String, _>("DATA_TYPE")?.to_lowercase(),
        })
    })
    .collect()
}

fn build_literal_select_query(
    schema: &str,
    table: &str,
    columns: &[ColumnSpec],
) -> Result<String, Error> {
    if columns.is_empty() {
        return Err(Error::protocol(format!(
            "Table `{table}` has no columns to mirror"
        )));
    }

    let select_list = columns
        .iter()
        .map(literal_select_expression)
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        "SELECT {select_list} FROM {}",
        qualified_table_name(schema, table)
    ))
}

fn build_insert_query(
    schema: &str,
    table: &str,
    columns: &[ColumnSpec],
    row: &sqlx::mysql::MySqlRow,
) -> Result<String, Error> {
    let column_list = columns
        .iter()
        .map(|column| quote_identifier(&column.name))
        .collect::<Vec<_>>()
        .join(", ");

    let values = (0..columns.len())
        .map(|index| row.try_get::<String, _>(index))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");

    Ok(format!(
        "INSERT INTO {} ({column_list}) VALUES ({values})",
        qualified_table_name(schema, table)
    ))
}

fn literal_select_expression(column: &ColumnSpec) -> String {
    let quoted_column = quote_identifier(&column.name);
    let alias = quote_identifier(&column.name);

    if is_binary_type(&column.data_type) {
        return format!(
            "CASE WHEN {quoted_column} IS NULL THEN 'NULL' \
             ELSE CONCAT('X''', HEX({quoted_column}), '''') END AS {alias}"
        );
    }

    if is_geometry_type(&column.data_type) {
        return format!(
            "CASE WHEN {quoted_column} IS NULL THEN 'NULL' \
             ELSE CONCAT('ST_GeomFromWKB(X''', HEX(ST_AsWKB({quoted_column})), ''')') END AS {alias}"
        );
    }

    format!("IFNULL(QUOTE({quoted_column}), 'NULL') AS {alias}")
}

fn qualified_table_name(schema: &str, table: &str) -> String {
    format!("{}.{}", quote_identifier(schema), quote_identifier(table))
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

fn is_binary_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" | "bit"
    )
}

fn is_geometry_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "geometry"
            | "point"
            | "linestring"
            | "polygon"
            | "multipoint"
            | "multilinestring"
            | "multipolygon"
            | "geometrycollection"
            | "geomcollection"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ColumnSpec, build_literal_select_query, literal_select_expression, quote_identifier,
    };

    #[test]
    fn quote_identifier_escapes_backticks() {
        assert_eq!(quote_identifier("order`items"), "`order``items`");
    }

    #[test]
    fn text_columns_use_quote() {
        let column = ColumnSpec {
            name: "status".into(),
            data_type: "varchar".into(),
        };

        assert_eq!(
            literal_select_expression(&column),
            "IFNULL(QUOTE(`status`), 'NULL') AS `status`"
        );
    }

    #[test]
    fn binary_columns_use_hex_literals() {
        let column = ColumnSpec {
            name: "payload".into(),
            data_type: "blob".into(),
        };

        assert_eq!(
            literal_select_expression(&column),
            "CASE WHEN `payload` IS NULL THEN 'NULL' ELSE CONCAT('X''', HEX(`payload`), '''') END AS `payload`"
        );
    }

    #[test]
    fn geometry_columns_use_wkb_literals() {
        let column = ColumnSpec {
            name: "shape".into(),
            data_type: "geometry".into(),
        };

        assert_eq!(
            literal_select_expression(&column),
            "CASE WHEN `shape` IS NULL THEN 'NULL' ELSE CONCAT('ST_GeomFromWKB(X''', HEX(ST_AsWKB(`shape`)), ''')') END AS `shape`"
        );
    }

    #[test]
    fn select_query_uses_qualified_table_name() {
        let columns = vec![ColumnSpec {
            name: "id".into(),
            data_type: "int".into(),
        }];

        assert_eq!(
            build_literal_select_query("primary-db", "users", &columns).unwrap(),
            "SELECT IFNULL(QUOTE(`id`), 'NULL') AS `id` FROM `primary-db`.`users`"
        );
    }
}

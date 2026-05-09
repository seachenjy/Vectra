use serde::{Deserialize, Serialize};
use sqlx::any::AnyRow;
use sqlx::pool::PoolOptions;
use sqlx::{Any, Column, Pool, Row, TypeInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("connection not found: {0}")]
    ConnectionNotFound(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    /// Connection identifier (user-defined name)
    pub name: String,
    /// Database type: sqlite, mysql, postgres
    pub db_type: String,
    /// Connection string / DSN
    /// SQLite: "sqlite:path/to/db.sqlite"
    /// MySQL: "mysql://user:pass@host:port/dbname"
    /// PostgreSQL: "postgres://user:pass@host:port/dbname"
    pub dsn: String,
    /// Max connections in pool
    pub max_connections: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultSet {
    /// Column names
    pub columns: Vec<String>,
    /// Column types
    pub column_types: Vec<String>,
    /// Rows as arrays of JSON values
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Total row count
    pub row_count: usize,
    /// Execution time in milliseconds
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub name: String,
    pub db_type: String,
    pub dsn_display: String,
    pub connected: bool,
}

/// Mask sensitive parts of DSN for display
fn mask_dsn(dsn: &str) -> String {
    if dsn.starts_with("sqlite:") {
        return dsn.to_string();
    }
    // Mask password in URL-style DSN: scheme://user:pass@host...
    if let Some(at_pos) = dsn.find('@') {
        if let Some(scheme_end) = dsn.find("://") {
            let prefix = &dsn[..scheme_end + 3];
            let rest = &dsn[scheme_end + 3..at_pos];
            let after = &dsn[at_pos..];
            if let Some(colon) = rest.find(':') {
                let user = &rest[..colon];
                return format!("{}{}:****{}", prefix, user, after);
            }
        }
    }
    dsn.to_string()
}

fn row_to_json_values(row: &AnyRow) -> Vec<serde_json::Value> {
    let cols = row.columns();
    let mut values = Vec::with_capacity(cols.len());
    for col in cols {
        let idx = col.ordinal();
        let type_name = col.type_info().name().to_uppercase();
        let val: serde_json::Value = match type_name.as_str() {
            "INTEGER" | "INT" | "INT4" | "INT8" | "BIGINT" | "SMALLINT" | "TINYINT" | "MEDIUMINT" => {
                row.try_get::<i64, _>(idx)
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null)
            }
            "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL" | "FLOAT4" | "FLOAT8" => {
                row.try_get::<f64, _>(idx)
                    .map(|v| serde_json::Value::from(v))
                    .unwrap_or(serde_json::Value::Null)
            }
            "BOOLEAN" | "BOOL" => {
                row.try_get::<bool, _>(idx)
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null)
            }
            _ => {
                // Fallback: try string
                row.try_get::<String, _>(idx)
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null)
            }
        };
        values.push(val);
    }
    values
}

struct PoolEntry {
    config: DbConfig,
    pool: Pool<Any>,
}

/// Manages multiple database connections
pub struct DatabaseManager {
    connections: Arc<RwLock<HashMap<String, PoolEntry>>>,
}

impl DatabaseManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to a database and register it under the given name
    pub async fn connect(&self, config: DbConfig) -> DbResult<()> {
        // Install default drivers for Any pool
        sqlx::any::install_default_drivers();

        let dsn = build_dsn(&config)?;
        let max_conn = config.max_connections.unwrap_or(5);

        let pool = PoolOptions::<Any>::new()
            .max_connections(max_conn)
            .connect(&dsn)
            .await?;

        let entry = PoolEntry {
            config: config.clone(),
            pool,
        };
        let mut conns = self.connections.write().await;
        conns.insert(config.name.clone(), entry);
        tracing::info!("database connected: {} ({})", config.name, config.db_type);
        Ok(())
    }

    /// Disconnect and remove a connection
    pub async fn disconnect(&self, name: &str) -> DbResult<()> {
        let mut conns = self.connections.write().await;
        if let Some(entry) = conns.remove(name) {
            entry.pool.close().await;
            tracing::info!("database disconnected: {}", name);
            Ok(())
        } else {
            Err(DbError::ConnectionNotFound(name.to_string()))
        }
    }

    /// List all connections
    pub async fn list_connections(&self) -> Vec<ConnectionInfo> {
        let conns = self.connections.read().await;
        conns.values().map(|e| ConnectionInfo {
            name: e.config.name.clone(),
            db_type: e.config.db_type.clone(),
            dsn_display: mask_dsn(&e.config.dsn),
            connected: !e.pool.is_closed(),
        }).collect()
    }

    /// Execute a SQL query and return results
    pub async fn query(&self, conn_name: &str, sql: &str) -> DbResult<QueryResultSet> {
        let conns = self.connections.read().await;
        let entry = conns.get(conn_name)
            .ok_or_else(|| DbError::ConnectionNotFound(conn_name.to_string()))?;

        let start = std::time::Instant::now();
        let rows: Vec<AnyRow> = sqlx::query(sql)
            .fetch_all(&entry.pool)
            .await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        if rows.is_empty() {
            return Ok(QueryResultSet {
                columns: vec![],
                column_types: vec![],
                rows: vec![],
                row_count: 0,
                elapsed_ms,
            });
        }

        let columns: Vec<String> = rows[0].columns().iter()
            .map(|c| c.name().to_string())
            .collect();
        let column_types: Vec<String> = rows[0].columns().iter()
            .map(|c| c.type_info().name().to_string())
            .collect();

        let data: Vec<Vec<serde_json::Value>> = rows.iter()
            .map(|r| row_to_json_values(r))
            .collect();
        let row_count = data.len();

        Ok(QueryResultSet {
            columns,
            column_types,
            rows: data,
            row_count,
            elapsed_ms,
        })
    }

    /// Execute a non-query SQL (INSERT, UPDATE, DELETE, CREATE, etc.)
    pub async fn execute(&self, conn_name: &str, sql: &str) -> DbResult<u64> {
        let conns = self.connections.read().await;
        let entry = conns.get(conn_name)
            .ok_or_else(|| DbError::ConnectionNotFound(conn_name.to_string()))?;

        let result = sqlx::query(sql)
            .execute(&entry.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// List tables in the connected database
    pub async fn list_tables(&self, conn_name: &str) -> DbResult<Vec<String>> {
        let conns = self.connections.read().await;
        let entry = conns.get(conn_name)
            .ok_or_else(|| DbError::ConnectionNotFound(conn_name.to_string()))?;

        let sql = match entry.config.db_type.as_str() {
            "sqlite" => "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            "mysql" => "SHOW TABLES",
            "postgres" => "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
            _ => return Err(DbError::InvalidConfig(format!("unsupported db_type: {}", entry.config.db_type))),
        };

        let rows: Vec<AnyRow> = sqlx::query(sql)
            .fetch_all(&entry.pool)
            .await?;

        let tables: Vec<String> = rows.iter()
            .filter_map(|r| r.try_get::<String, _>(0).ok())
            .collect();
        Ok(tables)
    }

    /// Describe a table (columns and types)
    pub async fn describe_table(&self, conn_name: &str, table: &str) -> DbResult<QueryResultSet> {
        let conns = self.connections.read().await;
        let entry = conns.get(conn_name)
            .ok_or_else(|| DbError::ConnectionNotFound(conn_name.to_string()))?;

        let sql = match entry.config.db_type.as_str() {
            "sqlite" => format!("PRAGMA table_info(\"{}\")", table),
            "mysql" => format!("DESCRIBE `{}`", table),
            "postgres" => format!(
                "SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
                table
            ),
            _ => return Err(DbError::InvalidConfig(format!("unsupported db_type: {}", entry.config.db_type))),
        };

        self.query(conn_name, &sql).await
    }

    /// Test whether a connection is alive
    pub async fn test_connection(&self, conn_name: &str) -> DbResult<bool> {
        let conns = self.connections.read().await;
        let entry = conns.get(conn_name)
            .ok_or_else(|| DbError::ConnectionNotFound(conn_name.to_string()))?;

        let sql = match entry.config.db_type.as_str() {
            "sqlite" => "SELECT 1",
            "mysql" => "SELECT 1",
            "postgres" => "SELECT 1",
            _ => "SELECT 1",
        };

        match sqlx::query(sql).fetch_one(&entry.pool).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Default for DatabaseManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the actual DSN string from config
fn build_dsn(config: &DbConfig) -> DbResult<String> {
    let dsn = config.dsn.trim().to_string();
    match config.db_type.as_str() {
        "sqlite" => {
            if dsn.starts_with("sqlite:") {
                Ok(dsn)
            } else {
                Ok(format!("sqlite:{}", dsn))
            }
        }
        "mysql" => {
            if dsn.starts_with("mysql://") {
                Ok(dsn)
            } else {
                Err(DbError::InvalidConfig("MySQL DSN must start with mysql://".into()))
            }
        }
        "postgres" => {
            if dsn.starts_with("postgres://") || dsn.starts_with("postgresql://") {
                Ok(dsn)
            } else {
                Err(DbError::InvalidConfig("PostgreSQL DSN must start with postgres://".into()))
            }
        }
        other => Err(DbError::InvalidConfig(format!("unsupported db_type: {}", other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_dsn() {
        assert_eq!(mask_dsn("sqlite:test.db"), "sqlite:test.db");
        assert_eq!(mask_dsn("mysql://root:secret@localhost:3306/mydb"), "mysql://root:****@localhost:3306/mydb");
        assert_eq!(mask_dsn("postgres://user:p4ss@host:5432/db"), "postgres://user:****@host:5432/db");
    }

    #[test]
    fn test_build_dsn() {
        let cfg = DbConfig {
            name: "test".into(),
            db_type: "sqlite".into(),
            dsn: "test.db".into(),
            max_connections: None,
        };
        assert_eq!(build_dsn(&cfg).unwrap(), "sqlite:test.db");

        let cfg2 = DbConfig {
            name: "test".into(),
            db_type: "mysql".into(),
            dsn: "mysql://root:pass@localhost/db".into(),
            max_connections: None,
        };
        assert_eq!(build_dsn(&cfg2).unwrap(), "mysql://root:pass@localhost/db");
    }
}

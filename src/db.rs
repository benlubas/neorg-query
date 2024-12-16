use std::path::Path;

use anyhow::bail;
use chrono::{DateTime, NaiveDateTime, Utc};
use itertools::Itertools;
use libsql::{params, Builder, Connection, OpenFlags, Rows};
use log::{error, info, trace};

use crate::doc_parser::ParsedDocument;

#[derive(Clone)]
pub struct DatabaseConnection {
    pub conn: Connection,
    pub read_conn: Connection,
    // TODO: store prepared queries maybe? I'm not really sure how those work or how much
    // performance they gain
}

impl DatabaseConnection {
    /// Create the database connection, and ensure that any tables we use exist
    pub async fn new(db_file: &Path) -> anyhow::Result<DatabaseConnection> {
        let db = Builder::new_local(db_file).build().await?;
        let read_db = Builder::new_local(db_file)
            .flags(OpenFlags::SQLITE_OPEN_READ_ONLY)
            .build()
            .await?;
        let conn = db.connect()?;
        let read_conn = read_db.connect()?;

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS docs
            (id INTEGER PRIMARY KEY,
            path VARCHAR(1024) UNIQUE NOT NULL,
            title TEXT,
            description TEXT,
            authors TEXT,
            created DATETIME,
            updated DATETIME,
            indexed DATETIME DEFAULT CURRENT_TIMESTAMP)"#,
            (),
        )
        .await?;

        conn.execute(
            r#"CREATE TRIGGER IF NOT EXISTS on_update
            AFTER UPDATE ON docs
            FOR EACH ROW
            BEGIN
            UPDATE docs SET indexed = CURRENT_TIMESTAMP WHERE id = old.id;
            END"#,
            (),
        )
        .await?;

        // if we try to add a cat that already exists for a file, we just do nothing and continue
        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS categories
            (id INTEGER PRIMARY KEY,
            file_id INTEGER,
            name VARCHAR(255) NOT NULL,
            FOREIGN KEY(file_id) REFERENCES docs(id),
            UNIQUE (file_id, name) ON CONFLICT IGNORE)"#,
            (),
        )
        .await?;

        Ok(DatabaseConnection { conn, read_conn })
    }

    /// Insert a doc or update it if it exists, returning the ID of the doc we just created.
    pub async fn insert_or_update_doc(&self, doc: &ParsedDocument) -> anyhow::Result<i64> {
        let mut rows = self.conn.query(
            "INSERT INTO docs (path, title, description, authors, created, updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title, description=excluded.description, authors=excluded.authors, updated=excluded.updated
             RETURNING id",
            doc.doc_params(),
        ).await?;

        let row = rows.next().await?;
        if let Some(row) = row {
            let id = *row
                .get_value(0)?
                .as_integer()
                .ok_or(anyhow::anyhow!("ID isn't an int"))?;

            self.conn
                .execute("DELETE FROM categories WHERE file_id = ?1", [id])
                .await?;

            if !doc.categories.is_empty() {
                let values = (0..doc.categories.len())
                    .map(|i| format!("(?{}, ?1)", i + 2))
                    .collect_vec()
                    .join(",");
                let cat_query = format!("INSERT INTO categories (name, file_id) VALUES {values}");
                let mut params = doc.categories.clone();
                params.insert(0, id.to_string());

                self.conn.execute(&cat_query, params).await?;
            }
            Ok(id)
        } else {
            bail!("Failed to fetch ID")
        }
    }

    /// Get the `updated` date that we've stored for the file, parse it into a DateTime
    pub async fn get_updated_date(&self, path: &str) -> anyhow::Result<DateTime<Utc>> {
        let mut rows = self
            .conn
            .query("SELECT indexed FROM docs WHERE path=?1", params![path])
            .await?;

        match rows.next().await {
            Ok(Some(row)) => match row.get_str(0) {
                Ok(date) => {
                    return Ok(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S")?.and_utc())
                }
                Err(e) => error!("Failed to get updated as string: {e:?}"),
            },
            Ok(None) => trace!("No rows (None) from query"),
            Err(e) => error!("Error from query: {e:?}"),
        }

        bail!("No `updated` entry for this path");
    }

    /// Execute a query in read only mode, return the result
    pub async fn user_query(
        &self,
        query: &str,
        params: impl params::IntoParams + std::fmt::Debug,
    ) -> anyhow::Result<Rows> {
        info!("Running Query: {query}");
        info!("With Params: {params:?}");
        Ok(self.read_conn.query(query, params).await?)
    }
}

pub mod util {
    use libsql::Row;

    /// Get a string value from a column, checking the types along the way
    pub fn gets_checked(row: &Row, column: i32) -> Option<String> {
        match row.column_type(column) {
            Ok(t) => match t {
                libsql::ValueType::Null => None,
                libsql::ValueType::Text => row.get_str(column).ok().map(String::from),
                _ => None,
            },
            Err(_) => None,
        }
    }
}

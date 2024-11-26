use std::path::Path;

use anyhow::bail;
use libsql::{Builder, Connection};

use crate::doc_parser::ParsedDocument;

pub struct DatabaseConnection {
    pub conn: Connection,
    // TODO: store prepared queries maybe? I'm not really sure how those work or how much
    // performance they gain
}

impl DatabaseConnection {
    /// Create the database connection, and ensure that any tables we use exist
    pub async fn new(db_file: &Path) -> anyhow::Result<DatabaseConnection> {
        let db = Builder::new_local(db_file).build().await?;
        let conn = db.connect()?;

        // should we parameterize on workspace? One table per workspace?
        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS docs
            (id INTEGER PRIMARY KEY,
            path VARCHAR(1024) UNIQUE NOT NULL,
            title TEXT,
            authors TEXT,
            created DATETIME,
            updated DATETIME)"#,
            (),
        )
        .await?;

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS categories
            (id INTEGER PRIMARY KEY,
            file_id INTEGER,
            name TEXT,
            FOREIGN KEY(file_id) REFERENCES docs(id))"#,
            (),
        )
        .await?;

        Ok(DatabaseConnection { conn })
    }

    /// Insert a doc or update it if it exists, returning the ID of the doc we just created.
    pub async fn insert_or_update_doc(&self, doc: &ParsedDocument) -> anyhow::Result<i64> {
        let mut rows = self.conn.query(
            "INSERT INTO docs (path, title, authors, created, updated)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title, authors=excluded.authors, updated=excluded.updated
             RETURNING id",
            doc.doc_params(),
        ).await?;

        let row = rows.next().await?;
        if let Some(row) = row {
            Ok(*row.get_value(0)?.as_integer().ok_or(anyhow::anyhow!("ID isn't an int"))?)
        } else {
            bail!("Failed to fetch ID")
        }
    }
}

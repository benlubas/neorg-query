use std::path::Path;

use anyhow::bail;
use chrono::{DateTime, NaiveDateTime, Utc};
use itertools::Itertools;
use libsql::{params, Builder, Connection, OpenFlags, Rows};
use log::{error, info, trace};
use serde::Deserialize;

use crate::doc_parser::{ParsedDocument, Task};

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
            r#"CREATE TRIGGER IF NOT EXISTS on_update_docs
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

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS tasks
            (task_id INTEGER PRIMARY KEY,
            file_id INTEGER NOT NULL,
            text TEXT NOT NULL,
            status VARCHAR(32) NOT NULL,
            due DATETIME,
            starts DATETIME,
            recurs DATETIME,
            priority VARCHAR(32),
            timestamp DATETIME,
            parent_id INTEGER,
            created DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(file_id) REFERENCES docs(id),
            FOREIGN KEY(parent_id) REFERENCES tasks(task_id),
            UNIQUE (file_id, parent_id, text) ON CONFLICT ABORT)"#,
            (),
        )
        .await?;

        Ok(DatabaseConnection { conn, read_conn })
    }

    /// Insert a doc or update it if it exists, returning the ID of the doc we just created.
    pub async fn insert_or_update_doc(&self, doc: &mut ParsedDocument) -> anyhow::Result<i64> {
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

            // push new categories
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

            add_tasks(&self.conn, doc, id).await?;

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

#[derive(Deserialize)]
struct TaskCreated {
    text: String,
    // this is a date, but we can treat it like a string for serialization purposes, and b/c we're
    // not manipulating it at all
    created: String,
    // same ^
    updated: String,
}

/// 1. fetch existing tasks for this file
/// 2. compare text between existing and new tasks, if they're the same, use the `created` date
///    from the existing task in the new task.
async fn add_tasks(
    conn: &Connection,
    doc: &mut ParsedDocument,
    doc_id: i64,
) -> anyhow::Result<()> {
    let mut rows = conn
        .query(
            "SELECT text, created, updated FROM tasks WHERE file_id = ?1",
            params![doc_id],
        )
        .await?;

    let mut tasks: Vec<TaskCreated> = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(task) = libsql::de::from_row(&row) {
            tasks.push(task);
        };
    }

    conn.execute("DELETE FROM tasks WHERE file_id = ?1", params![doc_id]).await?;

    // recurse through tasks, and search for matching text.
    // any that have the same text will keep their updated field
    async fn update_tasks(
        conn: &Connection,
        tasks: &mut Vec<Task>,
        existing_tasks: &Vec<TaskCreated>,
        parent: Option<i64>,
        file_id: i64,
    ) -> anyhow::Result<()> {
        for task in tasks {
            task.parent_id = parent;
            task.file_id = Some(file_id);
            for et in existing_tasks {
                if et.text == task.text {
                    task.created = Some(et.created.clone());
                    task.updated = Some(et.updated.clone());
                }
            }
            // I hate this more than you do. I promise. I hate all of this code.
            // I've considered switching off of SQLite b/c of some of this code. (I should really
            // just use diesel but that feels so heavy).
            let mut rows = conn.query(
                "INSERT INTO tasks
                    (text,
                    status,
                    due,
                    starts,
                    recurs,
                    priority,
                    timestamp,
                    parent_id,
                    created,
                    file_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 RETURNING task_id",
                task.task_params(),
            ).await?;
            let row = rows.next().await?;
            if let Some(row) = row {
                let id = *row
                    .get_value(0)?
                    .as_integer()
                    .ok_or(anyhow::anyhow!("ID isn't an int"))?;
                Box::pin(update_tasks(conn, &mut task.children, existing_tasks, Some(id), file_id)).await?;
            } else {
                error!("Failed to get ID for task {}, stopping. Children will not be added", task.text);
            }
        }
        Ok(())
    }

    update_tasks(conn, &mut doc.tasks, &tasks, None, doc_id).await?;

    Ok(())
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

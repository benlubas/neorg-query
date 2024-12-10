mod db;
mod doc_parser;
mod orchestrator;

use std::{fs::File, path::Path, sync::OnceLock};

use anyhow::anyhow;
use db::{util::gets_checked, DatabaseConnection};
use itertools::Itertools;
use log::info;
use mlua::prelude::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use simplelog::{CombinedLogger, WriteLogger};
use tokio::runtime::{self};

static DB: OnceLock<DatabaseConnection> = OnceLock::new();

static TOKIO: Lazy<runtime::Runtime> = Lazy::new(|| {
    runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("cannot start tokio runtime")
});

/// Initialize the Database connection, optionally perform the initial workspace index
/// Returns true on success, false on failure
async fn init(
    _: Lua,
    (database_path, workspace_path, do_index): (String, String, bool),
) -> LuaResult<bool> {
    let handle = TOKIO.handle();

    let res = handle
        .spawn(async move {
            let ws_path = Path::new(&workspace_path);
            let db = DatabaseConnection::new(Path::new(&database_path))
                .await
                .expect("failed to create DB connection");

            let _ = DB.set(db);
            if do_index {
                let db = DB.get().expect("failed to get DB in init (should not be possible)");
                orchestrator::index_workspace(ws_path, db).await
            } else {
                Ok(())
            }
        })
        .await;

    Ok(res.is_ok())
}

async fn index(_: Lua, path: String) -> LuaResult<bool> {
    let handle = TOKIO.handle();
    let db = DB.get().expect("failed to get DB in index");

    let p = Path::new(&path);
    if !p.exists() {
        return Err(anyhow!("Path doesn't exist").into_lua_err());
    }

    let res = handle
        .spawn(async move {
            let path = Path::new(&path);
            if path.is_file() {
                orchestrator::index_file(path, db).await
            } else {
                orchestrator::index_workspace(path, db).await
            }
        })
        .await;

    Ok(res.is_ok())
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryQueryResponse {
    path: String,
    title: Option<String>,
    description: Option<String>,
    created: Option<String>,
    updated: Option<String>,
}

async fn category_query(lua: Lua, categories: Vec<String>) -> LuaResult<Vec<LuaValue>> {
    let handle = TOKIO.handle();

    let res = handle.spawn(async move {
        if categories.is_empty() {
            // I feel like this will be slower, even though it's easier to write. I'm annoyed that just
            // bail! doesn't automatically type convert
            // (|| bail!("Need at least one category"))()?;
            return Err(anyhow!("Need at least one category").into_lua_err());
        }

        let db = DB.get().expect("failed to get DB in category query");
        let q = "SELECT path, title, description, created, updated FROM docs d JOIN categories c ON d.id = c.file_id WHERE "
            .to_string()
            + &(0..categories.len())
                .map(|i| format!("c.name = ?{}", i + 1))
                .join(" AND ");

        let mut rows = db.user_query(&q, categories).await?;
        let mut res = vec![];
        while let Ok(Some(row)) = rows.next().await {
            res.push(CategoryQueryResponse {
                path: gets_checked(&row, 0).expect("file didn't have a path"),
                title: gets_checked(&row, 1),
                description: gets_checked(&row, 2),
                created: gets_checked(&row, 3),
                updated: gets_checked(&row, 4),
            })
        }

        Ok(res)
    }).await;

    Ok(res
        .expect("cat query task failed")
        .expect("cat query task returned Err")
        .iter()
        .filter_map(|x| lua.to_value(&x).ok())
        .collect())
}

async fn all_categories(_lua: Lua, _: ()) -> LuaResult<Vec<String>> {
    let handle = TOKIO.handle();

    let res = handle
        .spawn(async move {
            let db = DB.get().expect("failed to get DB in all_categories");
            let q = "SELECT DISTINCT name FROM categories";

            let mut rows = db.user_query(q, ()).await?;
            let mut res = vec![];
            while let Ok(Some(row)) = rows.next().await {
                if let Some(name) = gets_checked(&row, 0) {
                    res.push(name);
                }
            }
            Ok::<Vec<String>, anyhow::Error>(res)
        })
        .await;

    Ok(res.expect("all cat task failed").expect("all cat task returned Err"))
}

// async fn greet(_lua: Lua, name: String) -> LuaResult<String> {
//     let _guard = TOKIO.enter();
//
//     tokio::time::sleep(std::time::Duration::from_secs(3)).await;
//     Ok(format!("Hello {name}!").to_string())
// }

#[mlua::lua_module]
fn libneorg_query(lua: &Lua) -> LuaResult<LuaTable> {
    CombinedLogger::init(vec![WriteLogger::new(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        File::create("/tmp/neorg-query.log").expect("failed to create log file"),
    )])
    .expect("failed to crate logger");
    log_panics::init();

    let exports = lua.create_table()?;
    exports.set("init", lua.create_async_function(init)?)?;
    exports.set("index", lua.create_async_function(index)?)?;
    exports.set("category_query", lua.create_async_function(category_query)?)?;
    exports.set("all_categories", lua.create_async_function(all_categories)?)?;

    exports.set(
        "PENDING",
        lua.create_async_function(|_, ()| async move {
            tokio::task::yield_now().await;
            Ok(())
        })?,
    )?;
    Ok(exports)
}

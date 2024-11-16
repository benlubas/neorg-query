mod search_engine;

use std::{sync::RwLock, thread};

use std::fs::File;

use log::{info, warn, LevelFilter};
use mlua::prelude::*;
use once_cell::sync::Lazy;
use simplelog::{CombinedLogger, Config, WriteLogger};
use tantivy::schema::Value;

use crate::search_engine::SearchEngine;

pub enum QueryType {
    Fulltext,
    Categories,
    Unknown(String),
}

struct QueryResult {
    score: f32,
    path: String,
}

impl mlua::IntoLua<'_> for QueryResult {
    fn into_lua(self, lua: &mlua::Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("score", self.score)?;
        table.set("path", self.path)?;
        Ok(mlua::Value::Table(table))
    }
}

static SEARCH_ENGINE: Lazy<RwLock<SearchEngine>> = Lazy::new(|| {
    let home = std::env::var("HOME").unwrap();
    RwLock::new(SearchEngine::new(format!("{home}/.local/share/nvim/")))
});

fn query(
    _: &Lua,
    (query_type, query_str): (&str, &str),
) -> LuaResult<Vec<QueryResult>> {
    info!("[QUERY] ({query_type:?}) `{query_str:?}`");
    let query_type = match query_type {
        "fulltext" => QueryType::Fulltext,
        "categories" => QueryType::Categories,
        s => QueryType::Unknown(s.to_string()),
    };
    if let Ok(search_engine) = SEARCH_ENGINE.read() {
        if let Ok(results) = search_engine.query(&query_type, query_str) {
            Ok(results
                .iter()
                .filter_map(|(score, r)| {
                    if let Ok(field) = search_engine.schema.get_field("path") {
                        r.get_all(field)
                            .next()
                            .unwrap()
                            .as_str()
                            .map(|path| QueryResult {
                                score: *score,
                                path: path.to_string(),
                            })
                    } else {
                        None
                    }
                })
                .collect())
        } else {
            Ok(vec![])
        }
    } else {
        Ok(vec![])
    }
}

fn index(_: &Lua, (ws_name, ws_path): (String, String)) -> LuaResult<()> {
    info!("[INDEX] start");

    info!("[INDEX] {ws_name}, {ws_path}");
    thread::spawn(move || {
        // Yeah I'm not stoked about this. But I think that it's fine. This is a data-race, but we
        // can't call into more than one function at a time. We're bound to one thread.
        if let Ok(mut search_engine) = SEARCH_ENGINE.write() {
            match search_engine.index(&ws_path, &ws_name) {
                Ok(_) => {
                    info!("[Index] Success");
                }
                Err(e) => {
                    info!("[Index] Failed with error: {e:?}");
                }
            };
        }
    });

    info!("[Index] returning");

    Ok(())
}

fn list_categories(_: &Lua, _: ()) -> LuaResult<Vec<String>> {
    // set the categories
    match SEARCH_ENGINE.read() {
        Ok(search_engine) => {
            if let Ok(cats) = search_engine.list_categories() {
                info!("[LIST CATS] result: {cats:?}");
                Ok(cats)
            } else {
                // TODO: should this be a different error?
                Ok(vec![])
            }
        }
        Err(e) => {
            warn!("[LIST CATS] Failed to aquire read lock on SEARCH_ENGINE: {e:?}");
            Ok(vec![])
        }
    }
}

#[mlua::lua_module]
fn libneorg_se(lua: &Lua) -> LuaResult<LuaTable> {
    // Yeah I'm not sure where else this log setup could even go
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        File::create("/tmp/neorg-SE.log").unwrap(),
    )])
    .unwrap();
    log_panics::init();

    let exports = lua.create_table()?;
    exports.set("query", lua.create_function(query)?)?;
    exports.set("index", lua.create_function(index)?)?;
    exports.set("list_categories", lua.create_function(list_categories)?)?;

    Ok(exports)
}

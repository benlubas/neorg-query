use std::{path::Path, sync::mpsc::channel};
use anyhow::Result;
use crate::DatabaseConnection;
use crate::ParsedDocument;
use ignore::{types::TypesBuilder, WalkBuilder};
use log::info;

pub async fn index_workspace(workspace_path: &Path, conn: DatabaseConnection) -> Result<DatabaseConnection> {
    info!("Indexing {workspace_path:?}\n...");

    let mut types = TypesBuilder::new();
    types.add("norg", "*.norg")?;
    let types = types.build()?;

    let (tx, rx) = channel::<Option<ParsedDocument>>();

    let x = conn.clone();
    let insert_job = tokio::spawn(async move {
        info!("insert job waiting");
        for doc in rx {
            if let Some(doc) = doc {
                // TODO: batch these eventually?
                x.clone().insert_or_update_doc(&doc).await.unwrap();
            } else {
                // sending None is the way to indicate that we're done.
                break;
            }
        }
    });

    // WalkBuilder::new(workspace_path).types(types).build_parallel().run(|| Box::new(|path| {
    //     if let Ok(path) = path {
    //         if let Ok(doc) = ParsedDocument::new(&path.path().to_string_lossy()) {
    //             let _ = tx.send(doc);
    //         };
    //     }
    //     WalkState::Continue
    // }));
    info!("Walking..");
    for entry in WalkBuilder::new(workspace_path).types(types).build().flatten() {
        if let Ok(doc) = ParsedDocument::new(&entry.path().to_string_lossy()) {
            tx.send(Some(doc)).unwrap();
        };
    }
    info!("Done walking");
    tx.send(None).unwrap();

    // I think this will infinitely loop b/c the receiver listens forever, how do we close it,
    // I guess we could change the type to option
    let _ = insert_job.await;

    Ok(conn)
}

// fn index(_: &Lua, (ws_name, ws_path): (String, String)) -> LuaResult<()> {
//     info!("[INDEX] start");
//
//     info!("[INDEX] {ws_name}, {ws_path}");
//     thread::spawn(move || {
//         // Yeah I'm not stoked about this. But I think that it's fine. This is a data-race, but we
//         // can't call into more than one function at a time. We're bound to one thread.
//         if let Ok(mut search_engine) = SEARCH_ENGINE.write() {
//             match search_engine.index(&ws_path, &ws_name) {
//                 Ok(_) => {
//                     info!("[Index] Success");
//                 }
//                 Err(e) => {
//                     info!("[Index] Failed with error: {e:?}");
//                 }
//             };
//         }
//     });
//
//     info!("[Index] returning");
//
//     Ok(())
// }
//
// fn list_categories(_: &Lua, _: ()) -> LuaResult<Vec<String>> {
//     // set the categories
//     match SEARCH_ENGINE.read() {
//         Ok(search_engine) => {
//             if let Ok(cats) = search_engine.list_categories() {
//                 info!("[LIST CATS] result: {cats:?}");
//                 Ok(cats)
//             } else {
//                 // TODO: should this be a different error?
//                 Ok(vec![])
//             }
//         }
//         Err(e) => {
//             warn!("[LIST CATS] Failed to aquire read lock on SEARCH_ENGINE: {e:?}");
//             Ok(vec![])
//         }
//     }
// }
//
// #[mlua::lua_module]
// fn libneorg_se(lua: &Lua) -> LuaResult<LuaTable> {
//     // Yeah I'm not sure where else this log setup could even go
//     CombinedLogger::init(vec![WriteLogger::new(
//         LevelFilter::Info,
//         Config::default(),
//         File::create("/tmp/neorg-SE.log").unwrap(),
//     )])
//     .unwrap();
//     log_panics::init();
//
//     let exports = lua.create_table()?;
//     exports.set("query", lua.create_function(query)?)?;
//     exports.set("index", lua.create_function(index)?)?;
//     exports.set("list_categories", lua.create_function(list_categories)?)?;
//
//     Ok(exports)
// }

use log::{info, LevelFilter};
use rpc_event_handler::EventHandler;

mod rpc_event_handler;
mod search_engine;
use simplelog::*;

use std::fs::File;

// fn main() {
//     CombinedLogger::init(vec![
//         // NOTE: this should only be enabled when running the application from the command line.
//         // having it enabled when running with neovim will absolutely destroy the rpc connection
//         // over stdI/O
//         // TermLogger::new(
//         //     LevelFilter::Info,
//         //     Config::default(),
//         //     TerminalMode::Mixed,
//         //     ColorChoice::Auto,
//         // ),
//         WriteLogger::new(
//             LevelFilter::Info,
//             Config::default(),
//             File::create("/tmp/neorg-SE.log").unwrap(),
//         ),
//     ])
//     .unwrap();
//     log_panics::init();
//
//     // use search_engine::ParsedDocument;
//     // let doc = ParsedDocument::from("/home/benlubas/notes/test1.norg");
//     // info!("Doc: {doc:?}");
//
//     let mut event_handler = EventHandler::new();
//     info!("[MAIN] Neorg-SE launched successfully\n");
//     event_handler.handle_events();
// }

use mlua::prelude::*;

fn hello(_: &Lua, name: String) -> LuaResult<()> {
    println!("hello, {}!", name);
    Ok(())
}

#[mlua::lua_module]
fn my_module(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("hello", lua.create_function(hello)?)?;
    Ok(exports)
}

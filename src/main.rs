use std::path::Path;

use db::DatabaseConnection;
use doc_parser::ParsedDocument;
use log::{info, LevelFilter};

use simplelog::*;

mod doc_parser;
mod db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        // WriteLogger::new(
        //     LevelFilter::Info,
        //     Config::default(),
        //     File::create("/tmp/neorg-query.log").unwrap(),
        // ),
    ])
    .unwrap();
    log_panics::init();

    info!("[MAIN] neorg-query running\n");

    // NOTE:
    // so we have a function that will parse a document, we have a function that will insert
    // a single document into the database or update that document
    // I've tested that the database file is persisted. Now we have to link everything up
    // - add categories to their own table
    // - method that parses an entire folder and adds all the files to the database
    // - potentially mlua

    // let mut doc = ParsedDocument::new("/home/benlubas/notes/test/test1.norg")?;
    // info!("Doc: {doc:?}");
    //
    let db = DatabaseConnection::new(Path::new("./test.sql")).await?;
    let mut rows = db.conn.query("select * from docs", ()).await?;
    let row = rows.next().await?;
    info!("{row:?}");
    // let id = db.insert_or_update_doc(&doc).await?;
    // info!("Id: {id:?}");
    //
    // doc.authors = vec!["new me".to_string()];
    // let id = db.insert_or_update_doc(&doc).await?;
    // info!("Should be same: {id:?}");

    Ok(())
}

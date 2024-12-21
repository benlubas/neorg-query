use crate::DatabaseConnection;
use crate::doc_parser::ParsedDocument;
use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use ignore::DirEntry;
use ignore::{types::TypesBuilder, WalkBuilder};
use log::info;
use std::convert::identity;
use std::time;
use std::{path::Path, sync::mpsc::channel};

/// Parse all the files in the workspace, skipping files that were last edited within a few ms of
/// the edited time we have for them. Streams reading, parsing, and inserting steps together
pub async fn index_workspace(
    path: &Path,
    conn: &DatabaseConnection,
) -> Result<()> {
    info!("Indexing {path:?}\n...");

    let start = time::Instant::now();

    let mut types = TypesBuilder::new();
    types.add("norg", "*.norg")?;
    let types = types.build()?;

    let (tx, rx) = channel::<Option<ParsedDocument>>();

    let x = conn.clone();
    let insert_job = tokio::spawn(async move {
        info!("insert job waiting");
        for doc in rx {
            if let Some(mut doc) = doc {
                x.clone().insert_or_update_doc(&mut doc).await.unwrap();
            } else {
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
    for entry in WalkBuilder::new(path)
        .types(types)
        .build()
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() || path.extension().is_none_or(|ext| ext != "norg") {
            continue;
        }
        let path = path.to_string_lossy();
        let stored_edit_time = conn.get_updated_date(&path).await;

        if stored_edit_time.is_ok_and(|t| !should_parse(&entry, t).is_ok_and(identity)) {
            info!("Skipping {path:?}");
            continue;
        };

        info!("Parsing {path:?}");
        // TODO: this parsing step is expensive, should spawn it into a task probably. But that
        // creates some lifetime problem
        if let Ok(doc) = ParsedDocument::new(&path) {
            tx.send(Some(doc)).unwrap();
        };
    }
    info!("Done walking");
    // this is the way we tell the insert job to stop listening
    tx.send(None).unwrap();

    let _ = insert_job.await;

    let end = time::Instant::now();
    info!("Index time: {:?}", end - start);

    Ok(())
}

// index a single file
pub async fn index_file(
    path: &Path,
    conn: &DatabaseConnection,
) -> Result<()> {
    assert!(path.is_file());

    if let Ok(mut doc) = ParsedDocument::new(path.to_str().unwrap()) {
        info!("{doc:?}");
        conn.insert_or_update_doc(&mut doc).await?;
    }

    Ok(())
}

fn should_parse(entry: &DirEntry, updated: DateTime<Utc>) -> anyhow::Result<bool> {
    let modified: i64 = entry
        .metadata()?
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    Ok(modified - updated.timestamp() > 3)
}

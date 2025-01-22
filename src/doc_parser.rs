use chrono::{DateTime, Utc};
use itertools::Itertools;
use libsql::params::IntoParams;
use libsql::params;
use log::{trace, warn};
use rust_norg::metadata::{parse_metadata, NorgMeta};
use rust_norg::{parse_tree, ParagraphSegment, ParagraphSegmentToken};
use rust_norg::{DetachedModifierExtension, NorgAST};
use std::io;

use std::fs;

use crate::norg_date;

// pub struct TaskItem {
//     content: String,
//     status: TodoStatus,
// }

#[derive(Debug)]
pub struct ParsedDocument {
    /// absolute file path
    pub path: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<String>,
    pub authors: Vec<String>,
    pub created_date: Option<String>,
    pub updated_date: Option<String>,
    // TODO: parse document body too?
    // / Paragraphs, untouched, still newlines, markup, links, etc.
    // / We will want a stripped version for searching against
    // pub paragraphs: Vec<String>,
    // // how do we want to store links? Should we just use the Neorg structure?
    // pub links: Vec<String>,
    /// Heading tasks only for now.
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub text: String,
    pub status: String,
    pub due: Option<DateTime<Utc>>,
    pub starts: Option<DateTime<Utc>>,
    pub recurs: Option<DateTime<Utc>>,
    pub timestamp: Option<DateTime<Utc>>,
    pub priority: Option<String>,
    pub children: Vec<Self>,
    /// Time we first encounter this task. (The existence of this field forces us to do a ton of
    /// extra work). Remains None until we're ready to insert them into the database
    /// this is a string representation of a DateTime
    pub created: Option<String>,
    pub updated: Option<String>,
    pub parent_id: Option<i64>,
    pub file_id: Option<i64>,
}

impl Task {
    pub fn new(text: String, status: String) -> Task {
        Task {
            text,
            status,
            due: None,
            starts: None,
            recurs: None,
            priority: None,
            timestamp: None,
            children: vec![],
            created: None,
            updated: None,
            parent_id: None,
            file_id: None,
        }
    }

    pub fn task_params(
        &self,
    ) -> impl IntoParams {
        let map_d = |date: DateTime<Utc>| date.timestamp();
        params![
            self.text.clone(),
            self.status.clone(),
            self.due.map(map_d),
            self.starts.map(map_d),
            self.recurs.map(map_d),
            self.priority.clone(),
            self.timestamp.map(map_d),
            self.parent_id,
            self.created.clone(),
            self.file_id.expect("Can't call to params without setting file id"),
        ]
    }
}

impl ParsedDocument {
    pub fn doc_params(&self) -> impl IntoParams {
        vec![
            Some(self.path.clone()),
            self.title.clone(),
            self.description.clone(),
            if self.authors.is_empty() {
                None
            } else {
                Some(self.authors.join(""))
            },
            self.created_date.clone(),
            self.updated_date.clone(),
        ]
    }
}

pub trait PlainText {
    fn plain_text(&self) -> String;
}

#[allow(clippy::collapsible_match)]
impl PlainText for ParagraphSegment {
    fn plain_text(&self) -> String {
        match self {
            ParagraphSegment::Token(x) => x.plain_text(),
            ParagraphSegment::AttachedModifier { content, .. } => {
                content.iter().map(ParagraphSegment::plain_text).collect()
            }
            ParagraphSegment::Link { description, .. } => match description {
                Some(d) => d.iter().map(ParagraphSegment::plain_text).collect(),
                None => String::from(""),
            },
            ParagraphSegment::AnchorDefinition { content, .. } => {
                content.iter().map(ParagraphSegment::plain_text).collect()
            }
            ParagraphSegment::Anchor { description, .. } => match description {
                Some(d) => d.iter().map(ParagraphSegment::plain_text).collect(),
                None => String::from(""),
            },
            ParagraphSegment::InlineLinkTarget(content) => {
                content.iter().map(ParagraphSegment::plain_text).collect()
            }
            ParagraphSegment::InlineVerbatim(tokens) => tokens
                .iter()
                .map(ParagraphSegmentToken::plain_text)
                .collect(),
            _ => String::from(""),
        }
    }
}

impl PlainText for ParagraphSegmentToken {
    fn plain_text(&self) -> String {
        match self {
            ParagraphSegmentToken::Text(s) => String::from(s),
            ParagraphSegmentToken::Whitespace => String::from(" "),
            ParagraphSegmentToken::Special(c) => c.to_string(),
            ParagraphSegmentToken::Escape(c) => c.to_string(),
        }
    }
}

/// fill in the metadata for a document
fn fill_meta(content: String, doc: &mut ParsedDocument) {
    if let Ok(NorgMeta::Object(meta)) = parse_metadata(&content) {
        let gets = |x: &str| {
            if let Some(NorgMeta::Str(s)) = meta.get(x) {
                Some(s.to_string())
            } else {
                None
            }
        };

        doc.title = gets("title");
        doc.description = gets("description");
        doc.created_date = gets("created");
        doc.updated_date = gets("updated");

        let geta = |x: &str| match meta.get(x) {
            Some(NorgMeta::Array(a)) => a
                .iter()
                .filter_map(|array_item| {
                    if let NorgMeta::Str(s) = array_item {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect_vec(),
            Some(NorgMeta::Str(s)) => vec![s.to_string()],
            _ => vec![],
        };

        doc.categories = geta("categories");
        doc.authors = geta("authors");
    } else {
        warn!("Failed to parse metadata for: {}", doc.path)
    }
}

fn examine_heading(
    _level: u16,
    title: Vec<ParagraphSegment>,
    extensions: Vec<DetachedModifierExtension>,
    content: Vec<NorgAST>,
    doc: &mut ParsedDocument,
) {
    let text: String = title.iter().map(|s| s.plain_text()).join("");

    // create a task with a temporarily empty status
    if !extensions.is_empty() {
        trace!("not empty {text}");
        trace!("extensions: {extensions:?}");
        let mut task = Task::new(text, String::from(""));
        for ext in extensions {
            match ext {
                DetachedModifierExtension::Todo(todo_status) => {
                    // NOTE: Surely there's a better way to do this...
                    // stack overflow seems to say proc macro.. which I don't want to bother with at
                    // the moment
                    task.status = match todo_status {
                        rust_norg::TodoStatus::Undone => "Undone",
                        rust_norg::TodoStatus::Done => "Done",
                        rust_norg::TodoStatus::NeedsClarification => "NeedsClarification",
                        rust_norg::TodoStatus::Paused => "Paused",
                        rust_norg::TodoStatus::Urgent => "Urgent",
                        rust_norg::TodoStatus::Recurring(_) => "Recurring",
                        rust_norg::TodoStatus::Pending => "Pending",
                        rust_norg::TodoStatus::Canceled => "Canceled",
                    }
                    .to_string()
                }
                DetachedModifierExtension::Priority(p) => {
                    task.priority = Some(p);
                }
                DetachedModifierExtension::Timestamp(t) => {
                    match norg_date::parse(&t) {
                        Ok(d) => task.timestamp = Some(d),
                        Err(e) => warn!("Failed to parse timestamp: {e}"),
                    }
                }
                DetachedModifierExtension::DueDate(t) => match norg_date::parse(&t) {
                    Ok(d) => task.due = Some(d),
                    Err(e) => warn!("Failed to parse due date: {e}"),
                },
                DetachedModifierExtension::StartDate(t) => match norg_date::parse(&t) {
                    Ok(d) => task.starts = Some(d),
                    Err(e) => warn!("Failed to parse due date: {e}"),
                },
            }
        }

        // some really messy logic to nest tasks without having to return anything (b/c this function
        // will eventually modify the doc in other ways).
        let before = doc.tasks.len();
        for node in content {
            descend(node, doc);
        }
        let tasks = doc.tasks.clone();
        let (existing, nested) = tasks.split_at(before);
        doc.tasks = existing.to_vec();

        task.children = nested.to_vec();
        doc.tasks.push(task);
    } else {
        for node in content {
            descend(node, doc);
        }
    }
}

fn descend(node: NorgAST, doc: &mut ParsedDocument) {
    match node {
        NorgAST::VerbatimRangedTag {
            name,
            parameters: _,
            content,
        } if name.len() == 2 && name[0] == "document" && name[1] == "meta" => {
            fill_meta(content, doc);
        }
        NorgAST::Heading {
            level,
            title,
            extensions,
            content,
        } => examine_heading(level, title, extensions, content, doc),
        _ => {}
    }
}

impl ParsedDocument {
    pub fn new(file_path: &str) -> io::Result<ParsedDocument> {
        let mut doc = ParsedDocument {
            title: None,
            description: None,
            categories: vec![],
            path: file_path.to_string(),
            authors: vec![],
            created_date: None,
            updated_date: None,
            tasks: vec![],
            // paragraphs: vec![],
            // links: vec![],
        };
        let contents = fs::read_to_string(file_path)?;

        let ast = parse_tree(&contents);
        if let Ok(ast) = ast {
            for node in ast {
                descend(node, &mut doc);
            }
        };
        trace!("{:?}", doc);
        Ok(doc)
    }
}

#[test]
fn parse_tasks() {
    // let doc = ParsedDocument::new("/home/benlubas/github/neorg-query/spec/tasks.norg");
    let doc = ParsedDocument::new("spec/tasks.norg");
    dbg!(&doc);
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    assert!(doc.tasks.len() == 5);
}

use dateparser::DateTimeUtc;
use itertools::Itertools;
use log::warn;
use rust_norg::metadata::{parse_metadata, NorgMeta};
use rust_norg::{parse_tree, ParagraphSegment, ParagraphSegmentToken};
use rust_norg::{DetachedModifierExtension, NorgAST};
use std::io;
use tokio::io::DuplexStream;

use std::fs;

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
    // /// Any header marked as a task that doesn't have a parent header with a task
    // pub task_headers: Vec<String>,
}

/// parse a neorg date into something else maybe
fn parse_neorg_date() {

}

#[derive(Debug)]
pub struct Task {
    pub text: String,
    pub status: String,
    pub due: Option<DateTimeUtc>,
    pub starts: Option<DateTimeUtc>,
    pub recurs: Option<DateTimeUtc>,
    pub timestamp: Option<DateTimeUtc>,
    pub priority: Option<String>,
    pub is_heading: bool,
    pub line_number: i32,
    pub children: Vec<Self>,
}
impl Task {
    pub fn new(text: String, status: String, is_heading: bool) -> Task {
        Task {
            text,
            status,
            due: None,
            starts: None,
            recurs: None,
            priority: None,
            timestamp: None,
            is_heading,
            line_number: -1,
            children: vec![],
        }
    }
}

impl ParsedDocument {
    pub fn doc_params(&self) -> Vec<Option<String>> {
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
    level: u16,
    title: Vec<ParagraphSegment>,
    extensions: Vec<DetachedModifierExtension>,
    content: Vec<NorgAST>,
    doc: &mut ParsedDocument,
) {
    let text: String = title.iter().map(|s| s.plain_text()).join("");
    // create a task with a temporarily empty status
    let mut task = Task::new(text, String::from(""), true);
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
                }.to_string()
            }
            DetachedModifierExtension::Priority(p) => {
                task.priority = Some(p);
            }
            DetachedModifierExtension::Timestamp(t) => {
                task.timestamp = Some(t.parse());
            }
            DetachedModifierExtension::DueDate(_) => todo!(),
            DetachedModifierExtension::StartDate(_) => todo!(),
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
            // TODO: traverse the tree and fill in fields

            for node in ast {
                descend(node, &mut doc);
            }
        };
        Ok(doc)
    }

    // consume other and merge it's data into ours. Don't need this if we're not parsing the
    // document body
    // pub fn merge(&mut self, other: &mut ParsedDocument) {
    //     if let None = self.title {
    //         self.title = other.title;
    //     }
    //     if let None = self.created_date {
    //         self.created_date = other.created_date;
    //     }
    //     if let None = self.updated_date {
    //         self.updated_date = other.updated_date;
    //     }
    //     self.categories.append(other.categories.as_mut());
    //     self.authors.append(other.authors.as_mut());
    //     // self.paragraphs.append(other.paragraphs.as_mut());
    //     // self.links.append(other.links.as_mut());
    //     // self.task_items.append(other.task_items.as_mut());
    //     // self.task_headers.append(other.task_headers.as_mut());
    // }
}

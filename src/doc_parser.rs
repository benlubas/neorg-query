use anyhow::anyhow;
use ignore::WalkBuilder;
use itertools::Itertools;
use libsql::params::IntoParams;
use rust_norg::metadata::{parse_metadata, NorgMeta};
use rust_norg::NorgAST;
use rust_norg::{parse_tree, ParagraphSegment, ParagraphSegmentToken, TodoStatus};
use std::io;
use std::path::Path;

use log::{error, info, warn};

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
    // /// Top level tasks only (with all sub content)
    // pub task_items: Vec<String>,
    // /// Any header marked as a task that doesn't have a parent header with a task
    // pub task_headers: Vec<String>,
}

impl ParsedDocument {
    pub fn doc_params(
        &self,
    ) -> (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
    ) {
        (
            self.path.clone(),
            self.title.clone(),
            self.authors.join(""),
            self.created_date.clone(),
            self.updated_date.clone(),
        )
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

impl ParsedDocument {
    pub fn new(file_path: &str) -> io::Result<ParsedDocument> {
        let contents = fs::read_to_string(file_path)?;

        let ast = parse_tree(&contents);
        info!("{ast:?}");

        let mut doc = ParsedDocument {
            title: None,
            categories: vec![],
            path: file_path.to_string(),
            authors: vec![],
            created_date: None,
            updated_date: None,
            // paragraphs: vec![],
            // links: vec![],
            // task_items: vec![],
            // task_headers: vec![],
        };
        if let Ok(ast) = ast {
            // TODO: traverse the tree and fill in fields

            for node in ast {
                match node {
                    NorgAST::VerbatimRangedTag {
                        name,
                        parameters: _,
                        content,
                    } if name.len() == 2 && name[0] == "document" && name[1] == "meta" => {
                        if let Ok(NorgMeta::Object(meta)) = parse_metadata(&content) {
                            let gets = |x: &str| {
                                if let Some(NorgMeta::Str(s)) = meta.get(x) {
                                    Some(s.to_string())
                                } else {
                                    None
                                }
                            };

                            doc.title = gets("title");
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
                            break;
                        }
                    }
                    _ => {}
                }
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

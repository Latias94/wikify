//! Template system for server-side rendering
//!
//! This module provides templates for server-side rendering using Askama.

use askama::Template;
use serde::Serialize;

/// Main page template
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub title: String,
    pub version: String,
    pub dev_mode: bool,
}

/// Repository page template
#[derive(Template)]
#[template(path = "repository.html")]
pub struct RepositoryTemplate {
    pub title: String,
    pub repository: String,
    pub session_id: String,
    pub is_indexed: bool,
}

/// Wiki page template
#[derive(Template)]
#[template(path = "wiki.html")]
pub struct WikiTemplate {
    pub title: String,
    pub wiki: WikiData,
    pub session_id: String,
}

/// Chat page template
#[derive(Template)]
#[template(path = "chat.html")]
pub struct ChatTemplate {
    pub title: String,
    pub repository: String,
    pub session_id: String,
}

/// Error page template
#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub title: String,
    pub error_code: u16,
    pub error_message: String,
}

/// Wiki data for templates
#[derive(Serialize)]
pub struct WikiData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub pages: Vec<WikiPageData>,
    pub sections: Vec<WikiSectionData>,
}

/// Wiki page data for templates
#[derive(Serialize)]
pub struct WikiPageData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub importance: String,
    pub reading_time: u32,
}

/// Wiki section data for templates
#[derive(Serialize)]
pub struct WikiSectionData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub pages: Vec<String>,
}

impl IndexTemplate {
    pub fn new(dev_mode: bool) -> Self {
        Self {
            title: "Wikify - AI-Powered Repository Documentation".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            dev_mode,
        }
    }
}

impl RepositoryTemplate {
    pub fn new(repository: String, session_id: String, is_indexed: bool) -> Self {
        Self {
            title: format!("Wikify - {}", repository),
            repository,
            session_id,
            is_indexed,
        }
    }
}

impl WikiTemplate {
    pub fn new(wiki: WikiData, session_id: String) -> Self {
        Self {
            title: format!("Wikify - {}", wiki.title),
            wiki,
            session_id,
        }
    }
}

impl ChatTemplate {
    pub fn new(repository: String, session_id: String) -> Self {
        Self {
            title: format!("Wikify Chat - {}", repository),
            repository,
            session_id,
        }
    }
}

impl ErrorTemplate {
    pub fn new(error_code: u16, error_message: String) -> Self {
        Self {
            title: format!("Error {} - Wikify", error_code),
            error_code,
            error_message,
        }
    }
}

/// Convert wikify_wiki::WikiStructure to WikiData
impl From<&wikify_wiki::WikiStructure> for WikiData {
    fn from(wiki: &wikify_wiki::WikiStructure) -> Self {
        Self {
            id: wiki.id.clone(),
            title: wiki.title.clone(),
            description: wiki.description.clone(),
            pages: wiki.pages.iter().map(WikiPageData::from).collect(),
            sections: wiki.sections.iter().map(WikiSectionData::from).collect(),
        }
    }
}

/// Convert wikify_wiki::WikiPage to WikiPageData
impl From<&wikify_wiki::WikiPage> for WikiPageData {
    fn from(page: &wikify_wiki::WikiPage) -> Self {
        Self {
            id: page.id.clone(),
            title: page.title.clone(),
            description: page.description.clone(),
            content: page.content.clone(),
            importance: format!("{:?}", page.importance),
            reading_time: page.reading_time,
        }
    }
}

/// Convert wikify_wiki::WikiSection to WikiSectionData
impl From<&wikify_wiki::WikiSection> for WikiSectionData {
    fn from(section: &wikify_wiki::WikiSection) -> Self {
        Self {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            pages: section.pages.clone(),
        }
    }
}

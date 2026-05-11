use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breadcrumb {
    pub title: String,
    pub url: String,
}

impl Breadcrumb {
    pub fn new(title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeoMeta {
    pub title: String,
    pub keywords: String,
    pub description: String,
    pub breadcrumbs: Vec<Breadcrumb>,
}

impl SeoMeta {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn with_keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = keywords.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_breadcrumb(mut self, title: impl Into<String>, url: impl Into<String>) -> Self {
        self.breadcrumbs.push(Breadcrumb::new(title, url));
        self
    }

    pub fn insert_into_context(&self, context: &mut Context) {
        context.insert("seo", self);

        // Legacy-compatible aliases for templates migrated from Danneo 1.x.
        context.insert("title", &self.title);
        context.insert("keywords", &self.keywords);
        context.insert("descript", &self.description);
        context.insert("breadcrumbs", &self.breadcrumbs);
    }
}

pub type PageContext = SeoMeta;

pub fn generate_slug(text: &str) -> String {
    let slug = slug::slugify(text);
    normalize_cpu_slug(&slug)
}

fn normalize_cpu_slug(slug: &str) -> String {
    let mut normalized = String::with_capacity(slug.len());
    let mut previous_dash = false;

    for ch in slug.chars().flat_map(char::to_lowercase) {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' {
            Some(ch)
        } else if ch == '-' {
            Some('-')
        } else {
            None
        };

        match next {
            Some('-') if !previous_dash && !normalized.is_empty() => {
                normalized.push('-');
                previous_dash = true;
            }
            Some('-') => {}
            Some(ch) => {
                normalized.push(ch);
                previous_dash = false;
            }
            None => {}
        }
    }

    normalized.trim_matches('-').to_string()
}

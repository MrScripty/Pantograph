//! Search index structures for Svelte documentation
//!
//! Provides data structures and logic for building a searchable index from markdown docs.

use std::path::Path;
use serde::{Deserialize, Serialize};

use super::docs::DocsError;

/// A searchable index of documentation entries
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchIndex {
    pub version: String,
    pub entries: Vec<IndexEntry>,
}

/// A single indexed documentation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Unique identifier (e.g., "runes/state")
    pub id: String,
    /// Display title (e.g., "$state")
    pub title: String,
    /// Section name (e.g., "Runes")
    pub section: String,
    /// Relative path to the markdown file
    pub path: String,
    /// Brief summary (first paragraph or extracted)
    pub summary: String,
    /// Keywords for searching
    pub keywords: Vec<String>,
    /// Full content for detailed search
    pub content: String,
}

impl SearchIndex {
    /// Build a search index from a directory of markdown documentation
    pub fn build_from_docs(docs_dir: &Path) -> Result<Self, DocsError> {
        let mut entries = Vec::new();

        // Walk through all directories
        if docs_dir.exists() {
            Self::collect_entries(docs_dir, docs_dir, &mut entries)?;
        }

        Ok(SearchIndex {
            version: "5.0.0".to_string(),
            entries,
        })
    }

    fn collect_entries(
        dir: &Path,
        base_dir: &Path,
        entries: &mut Vec<IndexEntry>,
    ) -> Result<(), DocsError> {
        let read_dir = std::fs::read_dir(dir).map_err(DocsError::Io)?;

        for entry in read_dir {
            let entry = entry.map_err(DocsError::Io)?;
            let path = entry.path();

            if path.is_dir() {
                Self::collect_entries(&path, base_dir, entries)?;
            } else if path.extension().map_or(false, |ext| ext == "md") {
                if let Ok(index_entry) = Self::parse_doc_file(&path, base_dir) {
                    entries.push(index_entry);
                }
            }
        }

        Ok(())
    }

    fn parse_doc_file(file_path: &Path, base_dir: &Path) -> Result<IndexEntry, DocsError> {
        let content = std::fs::read_to_string(file_path).map_err(DocsError::Io)?;
        let relative_path = file_path
            .strip_prefix(base_dir)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Extract section from directory name
        let section = file_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| Self::format_section_name(&s.to_string_lossy()))
            .unwrap_or_else(|| "General".to_string());

        // Parse frontmatter if present
        let (title, main_content) = Self::parse_frontmatter(&content);

        // Extract title from content if not in frontmatter
        let title = title.unwrap_or_else(|| {
            Self::extract_title_from_content(&main_content)
                .unwrap_or_else(|| {
                    file_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                })
        });

        // Generate ID from path
        let id = relative_path
            .trim_end_matches(".md")
            .replace('\\', "/")
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                // Remove number prefix like "01-"
                if s.len() > 3 && s.chars().nth(2) == Some('-') {
                    &s[3..]
                } else {
                    s
                }
            })
            .collect::<Vec<_>>()
            .join("/");

        // Extract summary (first paragraph)
        let summary = Self::extract_summary(&main_content);

        // Extract keywords
        let keywords = Self::extract_keywords(&title, &main_content);

        Ok(IndexEntry {
            id,
            title,
            section,
            path: relative_path,
            summary,
            keywords,
            content: main_content,
        })
    }

    fn format_section_name(name: &str) -> String {
        // Remove number prefix and format nicely
        let name = if name.len() > 3 && name.chars().nth(2) == Some('-') {
            &name[3..]
        } else {
            name
        };

        name.split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn parse_frontmatter(content: &str) -> (Option<String>, String) {
        if !content.starts_with("---") {
            return (None, content.to_string());
        }

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return (None, content.to_string());
        }

        let frontmatter = parts[1];
        let main_content = parts[2].trim().to_string();

        // Extract title from frontmatter
        let title = frontmatter
            .lines()
            .find(|line| line.starts_with("title:"))
            .map(|line| {
                line.trim_start_matches("title:")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string()
            });

        (title, main_content)
    }

    fn extract_title_from_content(content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                return Some(trimmed[2..].trim().to_string());
            }
        }
        None
    }

    fn extract_summary(content: &str) -> String {
        let mut in_code_block = false;
        let mut paragraphs = Vec::new();
        let mut current = String::new();

        for line in content.lines() {
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                continue;
            }

            let trimmed = line.trim();

            // Skip headers
            if trimmed.starts_with('#') {
                continue;
            }

            // Skip empty lines but end current paragraph
            if trimmed.is_empty() {
                if !current.is_empty() {
                    paragraphs.push(current.trim().to_string());
                    current.clear();
                    if paragraphs.len() >= 2 {
                        break;
                    }
                }
                continue;
            }

            current.push_str(trimmed);
            current.push(' ');
        }

        if !current.is_empty() && paragraphs.len() < 2 {
            paragraphs.push(current.trim().to_string());
        }

        let summary = paragraphs.join(" ");

        // Truncate if too long
        if summary.len() > 500 {
            format!("{}...", &summary[..497])
        } else {
            summary
        }
    }

    fn extract_keywords(title: &str, content: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // Add title words
        for word in title.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '$');
            if !clean.is_empty() && clean.len() > 1 {
                keywords.push(clean.to_lowercase());
            }
        }

        // Look for Svelte-specific patterns
        let svelte_patterns = [
            "$state", "$derived", "$effect", "$props", "$bindable", "$inspect", "$host",
            "onclick", "onchange", "oninput", "onsubmit", "onkeydown",
            "bind:", "class:", "style:", "use:", "transition:", "animate:",
            "#if", "#each", "#await", "#key", "@html", "@const", "@debug", "@render",
            "<svelte:", "runes", "snippet", "render",
        ];

        for pattern in svelte_patterns {
            if content.contains(pattern) {
                keywords.push(pattern.to_lowercase());
            }
        }

        // Deduplicate
        keywords.sort();
        keywords.dedup();

        keywords
    }
}


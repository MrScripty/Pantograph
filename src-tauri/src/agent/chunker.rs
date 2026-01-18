//! Document chunking module for splitting markdown documents into embeddable chunks.
//!
//! This module provides rule-based chunking of markdown documents, splitting at H2/H3
//! header boundaries while preserving context through header breadcrumbs.

use crate::agent::types::{ChunkPreview, ChunkPreviewItem, DocChunk};

/// Configuration for chunking behavior
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Minimum chunk size in characters (avoid tiny chunks)
    pub min_chunk_size: usize,
    /// Maximum chunk size in characters (split large sections)
    pub max_chunk_size: usize,
    /// Include header breadcrumb in chunk content
    pub include_header_context: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: 200,
            max_chunk_size: 4000,
            include_header_context: true,
        }
    }
}

/// Represents a parsed section from markdown
#[derive(Debug, Clone)]
pub struct ParsedSection {
    /// Header level (1-6 for h1-h6), 0 for content before any header
    pub level: u8,
    /// Header title text
    pub title: String,
    /// Content under this header (not including sub-headers)
    pub content: String,
    /// Line number where this section ends (exclusive, 0-indexed)
    pub end_line: usize,
    /// Parent header indices for building breadcrumbs
    pub parent_indices: Vec<usize>,
}

/// Parse markdown into a flat list of sections with hierarchy info
pub fn parse_markdown_structure(content: &str) -> Vec<ParsedSection> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections: Vec<ParsedSection> = Vec::new();
    let mut header_stack: Vec<(u8, usize)> = Vec::new(); // (level, section_index)

    let mut current_section: Option<ParsedSection> = None;
    let mut current_content = String::new();

    for (line_num, line) in lines.iter().enumerate() {
        if let Some((level, title)) = parse_header_line(line) {
            // Finish previous section if any
            if let Some(mut section) = current_section.take() {
                section.content = current_content.trim().to_string();
                section.end_line = line_num;
                sections.push(section);
                current_content = String::new();
            } else if !current_content.trim().is_empty() {
                // Content before any header
                sections.push(ParsedSection {
                    level: 0,
                    title: String::new(),
                    content: current_content.trim().to_string(),
                    end_line: line_num,
                    parent_indices: Vec::new(),
                });
                current_content = String::new();
            }

            // Update header stack - pop headers of same or lower level
            while let Some((stack_level, _)) = header_stack.last() {
                if *stack_level >= level {
                    header_stack.pop();
                } else {
                    break;
                }
            }

            // Build parent indices from current stack
            let parent_indices: Vec<usize> = header_stack.iter().map(|(_, idx)| *idx).collect();

            // Start new section
            let section_index = sections.len();
            current_section = Some(ParsedSection {
                level,
                title: title.to_string(),
                content: String::new(),
                end_line: 0, // Will be set when section ends
                parent_indices,
            });

            // Push this header onto the stack
            header_stack.push((level, section_index));
        } else {
            // Regular content line
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Don't forget the last section
    if let Some(mut section) = current_section.take() {
        section.content = current_content.trim().to_string();
        section.end_line = lines.len();
        sections.push(section);
    } else if !current_content.trim().is_empty() {
        sections.push(ParsedSection {
            level: 0,
            title: String::new(),
            content: current_content.trim().to_string(),
            end_line: lines.len(),
            parent_indices: Vec::new(),
        });
    }

    sections
}

/// Parse a line to check if it's a markdown header
/// Returns (level, title) if it is, None otherwise
fn parse_header_line(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }

    let mut level = 0u8;
    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else {
            break;
        }
    }

    if level > 6 || level == 0 {
        return None;
    }

    // Get the title after the # symbols
    let title = trimmed[level as usize..].trim();
    if title.is_empty() {
        return None;
    }

    Some((level, title))
}

/// Build header breadcrumb from section hierarchy
fn build_header_context(sections: &[ParsedSection], section: &ParsedSection) -> String {
    let mut parts: Vec<&str> = Vec::new();

    for &parent_idx in &section.parent_indices {
        if let Some(parent) = sections.get(parent_idx) {
            if !parent.title.is_empty() {
                parts.push(&parent.title);
            }
        }
    }

    if !section.title.is_empty() {
        parts.push(&section.title);
    }

    parts.join(" > ")
}

/// Check if content contains code blocks
fn has_code_blocks(content: &str) -> bool {
    content.contains("```")
}

/// Convert parsed sections into chunks
pub fn sections_to_chunks(
    sections: &[ParsedSection],
    doc_id: &str,
    doc_title: &str,
    section_name: &str,
    config: &ChunkConfig,
) -> Vec<DocChunk> {
    let mut chunks: Vec<DocChunk> = Vec::new();
    let mut pending_content = String::new();
    let mut pending_context = String::new();
    let mut pending_title = String::new();
    let mut pending_has_code = false;

    for section in sections {
        // We chunk at H2 and H3 boundaries
        let is_chunk_boundary = section.level == 2 || section.level == 3;

        if is_chunk_boundary && !pending_content.trim().is_empty() {
            // Finalize previous chunk
            let chunk_content = if config.include_header_context && !pending_context.is_empty() {
                format!("Context: {}\n\n{}", pending_context, pending_content.trim())
            } else {
                pending_content.trim().to_string()
            };

            if chunk_content.len() >= config.min_chunk_size || chunks.is_empty() {
                chunks.push(DocChunk {
                    id: format!("{}#chunk{}", doc_id, chunks.len()),
                    doc_id: doc_id.to_string(),
                    title: pending_title.clone(),
                    doc_title: doc_title.to_string(),
                    section: section_name.to_string(),
                    chunk_index: chunks.len() as u32,
                    total_chunks: 0, // Will be set later
                    header_context: pending_context.clone(),
                    content: chunk_content,
                    has_code: pending_has_code,
                });
            }

            pending_content = String::new();
            pending_has_code = false;
        }

        // Update context for new section
        if is_chunk_boundary || section.level == 1 {
            pending_context = build_header_context(sections, section);
            pending_title = section.title.clone();
        }

        // Add section content
        if !section.title.is_empty() {
            let header_prefix = "#".repeat(section.level as usize);
            pending_content.push_str(&format!("{} {}\n\n", header_prefix, section.title));
        }
        if !section.content.is_empty() {
            pending_content.push_str(&section.content);
            pending_content.push_str("\n\n");
        }

        if has_code_blocks(&section.content) {
            pending_has_code = true;
        }

        // Check if we need to split due to max size
        if pending_content.len() > config.max_chunk_size {
            // Split at paragraph boundaries
            let split_chunks = split_large_content(&pending_content, config.max_chunk_size);
            for (i, split_content) in split_chunks.into_iter().enumerate() {
                let chunk_content = if config.include_header_context && !pending_context.is_empty() && i == 0 {
                    format!("Context: {}\n\n{}", pending_context, split_content.trim())
                } else {
                    split_content.trim().to_string()
                };

                if chunk_content.len() >= config.min_chunk_size || chunks.is_empty() {
                    chunks.push(DocChunk {
                        id: format!("{}#chunk{}", doc_id, chunks.len()),
                        doc_id: doc_id.to_string(),
                        title: pending_title.clone(),
                        doc_title: doc_title.to_string(),
                        section: section_name.to_string(),
                        chunk_index: chunks.len() as u32,
                        total_chunks: 0,
                        header_context: pending_context.clone(),
                        content: chunk_content,
                        has_code: has_code_blocks(&split_content),
                    });
                }
            }
            pending_content = String::new();
            pending_has_code = false;
        }
    }

    // Don't forget the last pending content
    if !pending_content.trim().is_empty() {
        let chunk_content = if config.include_header_context && !pending_context.is_empty() {
            format!("Context: {}\n\n{}", pending_context, pending_content.trim())
        } else {
            pending_content.trim().to_string()
        };

        // Always include the last chunk even if small
        chunks.push(DocChunk {
            id: format!("{}#chunk{}", doc_id, chunks.len()),
            doc_id: doc_id.to_string(),
            title: pending_title,
            doc_title: doc_title.to_string(),
            section: section_name.to_string(),
            chunk_index: chunks.len() as u32,
            total_chunks: 0,
            header_context: pending_context,
            content: chunk_content,
            has_code: pending_has_code,
        });
    }

    // Update total_chunks in all chunks
    let total = chunks.len() as u32;
    for chunk in &mut chunks {
        chunk.total_chunks = total;
    }

    chunks
}

/// Split large content at paragraph boundaries
fn split_large_content(content: &str, max_size: usize) -> Vec<String> {
    let paragraphs: Vec<&str> = content.split("\n\n").collect();
    let mut result: Vec<String> = Vec::new();
    let mut current = String::new();

    for para in paragraphs {
        if current.len() + para.len() + 2 > max_size && !current.is_empty() {
            result.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(para);
    }

    if !current.is_empty() {
        result.push(current);
    }

    if result.is_empty() {
        result.push(content.to_string());
    }

    result
}

/// Main entry point: chunk a document
pub fn chunk_document(
    doc_id: &str,
    doc_title: &str,
    section: &str,
    content: &str,
    config: &ChunkConfig,
) -> Vec<DocChunk> {
    let sections = parse_markdown_structure(content);
    sections_to_chunks(&sections, doc_id, doc_title, section, config)
}

/// Generate a preview of how a document will be chunked
pub fn preview_chunks(
    doc_id: &str,
    doc_title: &str,
    section: &str,
    content: &str,
    config: &ChunkConfig,
) -> ChunkPreview {
    let sections = parse_markdown_structure(content);
    let chunks = sections_to_chunks(&sections, doc_id, doc_title, section, config);

    // Build preview items with line numbers
    let lines: Vec<&str> = content.lines().collect();
    let mut preview_items: Vec<ChunkPreviewItem> = Vec::new();

    for chunk in &chunks {
        // Find approximate line numbers by searching for chunk title in content
        let (start_line, end_line) = find_chunk_lines(&lines, &chunk.title, &chunk.content);

        let content_preview = if chunk.content.len() > 200 {
            format!("{}...", &chunk.content[..200])
        } else {
            chunk.content.clone()
        };

        preview_items.push(ChunkPreviewItem {
            chunk_index: chunk.chunk_index,
            title: chunk.title.clone(),
            header_context: chunk.header_context.clone(),
            content_preview,
            full_content: chunk.content.clone(),
            char_count: chunk.content.len(),
            has_code: chunk.has_code,
            start_line,
            end_line,
        });
    }

    ChunkPreview {
        doc_id: doc_id.to_string(),
        doc_title: doc_title.to_string(),
        total_chunks: chunks.len(),
        chunks: preview_items,
    }
}

/// Find approximate line numbers for a chunk
fn find_chunk_lines(lines: &[&str], title: &str, content: &str) -> (usize, usize) {
    let mut start_line = 0;

    // Try to find the header line
    if !title.is_empty() {
        for (i, line) in lines.iter().enumerate() {
            if line.contains(&title) && line.trim().starts_with('#') {
                start_line = i;
                break;
            }
        }
    }

    // Estimate end line based on content length
    let content_lines = content.lines().count();
    let end_line = (start_line + content_lines).min(lines.len());

    (start_line, end_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_line() {
        assert_eq!(parse_header_line("# Title"), Some((1, "Title")));
        assert_eq!(parse_header_line("## Section"), Some((2, "Section")));
        assert_eq!(parse_header_line("### Subsection"), Some((3, "Subsection")));
        assert_eq!(parse_header_line("Not a header"), None);
        assert_eq!(parse_header_line("#"), None);
        assert_eq!(parse_header_line("####### Too many"), None);
    }

    #[test]
    fn test_parse_markdown_structure() {
        let content = r#"# Main Title

Introduction paragraph.

## Section One

Content of section one.

### Subsection

Subsection content.

## Section Two

Content of section two.
"#;

        let sections = parse_markdown_structure(content);
        assert_eq!(sections.len(), 5);
        assert_eq!(sections[0].title, "Main Title");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[1].title, "Section One");
        assert_eq!(sections[1].level, 2);
        assert_eq!(sections[2].title, "Subsection");
        assert_eq!(sections[2].level, 3);
        assert_eq!(sections[3].title, "Section Two");
        assert_eq!(sections[3].level, 2);
    }

    #[test]
    fn test_chunk_document() {
        let content = r#"# Main Title

Introduction paragraph.

## Section One

Content of section one with some details.

### Subsection A

Subsection A content.

### Subsection B

Subsection B content.

## Section Two

Content of section two.
"#;

        let config = ChunkConfig::default();
        let chunks = chunk_document("test-doc", "Test Document", "test", content, &config);

        // Should have multiple chunks based on H2/H3 boundaries
        assert!(!chunks.is_empty());

        // All chunks should have correct doc_id
        for chunk in &chunks {
            assert!(chunk.doc_id == "test-doc");
            assert!(chunk.doc_title == "Test Document");
        }
    }

    #[test]
    fn test_code_block_detection() {
        let content = r#"## Example

Here's some code:

```javascript
const x = 1;
```

More text.
"#;

        let config = ChunkConfig::default();
        let chunks = chunk_document("test", "Test", "test", content, &config);

        // Should detect code blocks
        let has_code_chunk = chunks.iter().any(|c| c.has_code);
        assert!(has_code_chunk);
    }
}

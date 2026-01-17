use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use super::error::ToolError;
use crate::agent::docs::DocsManager;
use crate::agent::docs_search::{search_docs, DocSearchOutput};
use crate::agent::rag::SharedRagManager;
use crate::agent::types::DocChunk;

// ============================================================================
// SearchSvelteDocsTool - Search Svelte 5 documentation (keyword-based)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SearchSvelteDocsArgs {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    3
}

#[derive(Clone)]
pub struct SearchSvelteDocsTool {
    docs_manager: Arc<DocsManager>,
}

impl SearchSvelteDocsTool {
    pub fn new(docs_manager: Arc<DocsManager>) -> Self {
        Self { docs_manager }
    }
}

impl Tool for SearchSvelteDocsTool {
    const NAME: &'static str = "search_svelte_docs";
    type Error = ToolError;
    type Args = SearchSvelteDocsArgs;
    type Output = DocSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search Svelte 5 documentation for syntax, APIs, and best practices. \
                          Use this when you need to verify Svelte 5 runes syntax ($state, $derived, $effect, $props), \
                          event handlers (onclick, onmouseenter), component patterns, or fix errors.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (e.g., '$state', 'event handlers', 'props', 'onclick')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 3)",
                        "default": 3
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        log::debug!("[search_svelte_docs] Searching for: {}", args.query);

        // Check if docs are available (does not auto-download)
        if let Err(e) = self.docs_manager.ensure_docs_available().await {
            // Return empty results with a note instead of failing
            // This allows the agent to continue without docs
            log::warn!("[search_svelte_docs] Docs not available: {}", e);
            return Ok(DocSearchOutput {
                query: args.query,
                total_matches: 0,
                results: vec![],
            });
        }

        // Load search index
        let index = match self.docs_manager.load_index() {
            Ok(idx) => idx,
            Err(e) => {
                log::warn!("[search_svelte_docs] Failed to load index: {}", e);
                return Ok(DocSearchOutput {
                    query: args.query,
                    total_matches: 0,
                    results: vec![],
                });
            }
        };

        log::debug!(
            "[search_svelte_docs] Loaded index with {} entries, searching...",
            index.entries.len()
        );

        // Perform fuzzy search
        let results = search_docs(&index, &args.query, args.limit);

        log::debug!(
            "[search_svelte_docs] Found {} results for query '{}'",
            results.len(),
            args.query
        );

        Ok(DocSearchOutput {
            query: args.query,
            total_matches: results.len(),
            results,
        })
    }
}

// ============================================================================
// SearchSvelteDocsVectorTool - Semantic search using LanceDB vectors
// ============================================================================

/// Output structure for the vector search tool
#[derive(Debug, Serialize)]
pub struct VectorDocSearchOutput {
    /// The original query
    pub query: String,
    /// Search results ordered by relevance
    pub results: Vec<VectorDocResult>,
    /// Total number of matches found
    pub total_matches: usize,
}

/// A vector search result with chunk details
#[derive(Debug, Serialize)]
pub struct VectorDocResult {
    /// Document title
    pub doc_title: String,
    /// Chunk/section title
    pub title: String,
    /// Section name
    pub section: String,
    /// Header context (breadcrumb path)
    pub header_context: String,
    /// Full content of the chunk
    pub content: String,
    /// Whether this chunk contains code examples
    pub has_code: bool,
}

impl From<DocChunk> for VectorDocResult {
    fn from(chunk: DocChunk) -> Self {
        Self {
            doc_title: chunk.doc_title,
            title: chunk.title,
            section: chunk.section,
            header_context: chunk.header_context,
            content: chunk.content,
            has_code: chunk.has_code,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchSvelteDocsVectorArgs {
    pub query: String,
    #[serde(default = "default_vector_search_limit")]
    pub limit: usize,
}

fn default_vector_search_limit() -> usize {
    3
}

#[derive(Clone)]
pub struct SearchSvelteDocsVectorTool {
    rag_manager: SharedRagManager,
}

impl SearchSvelteDocsVectorTool {
    pub fn new(rag_manager: SharedRagManager) -> Self {
        Self { rag_manager }
    }
}

impl Tool for SearchSvelteDocsVectorTool {
    const NAME: &'static str = "search_svelte_docs_vector";
    type Error = ToolError;
    type Args = SearchSvelteDocsVectorArgs;
    type Output = VectorDocSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Semantic search Svelte 5 documentation using vector embeddings. \
                          More accurate than keyword search for finding conceptually related content. \
                          Use this when you need to understand Svelte 5 concepts, fix errors, or find related documentation. \
                          Requires documentation to be indexed first.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query describing what you're looking for (e.g., 'how to declare reactive props', 'event handler syntax')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 3)",
                        "default": 3
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let rag_guard = self.rag_manager.read().await;

        // Perform semantic vector search
        let chunks = rag_guard
            .search(&args.query, args.limit)
            .await
            .map_err(|e| ToolError::Validation(format!("Vector search failed: {}. Make sure documentation is indexed.", e)))?;

        let results: Vec<VectorDocResult> = chunks.into_iter().map(VectorDocResult::from).collect();
        let total = results.len();

        Ok(VectorDocSearchOutput {
            query: args.query,
            results,
            total_matches: total,
        })
    }
}

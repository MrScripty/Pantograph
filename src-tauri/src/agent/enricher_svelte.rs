//! Svelte Documentation Enricher
//!
//! Enriches validation errors with relevant Svelte 5 documentation
//! using vector search (RAG) to find the most relevant doc chunks.

use super::enricher::{ErrorCategory, ErrorEnricher};
use super::rag::SharedRagManager;
use async_trait::async_trait;

/// Enricher that adds relevant Svelte 5 documentation to errors
pub struct SvelteDocsEnricher {
    rag_manager: SharedRagManager,
}

impl SvelteDocsEnricher {
    pub fn new(rag_manager: SharedRagManager) -> Self {
        Self { rag_manager }
    }
}

#[async_trait]
impl ErrorEnricher for SvelteDocsEnricher {
    fn name(&self) -> &'static str {
        "SvelteDocsEnricher"
    }

    fn handles(&self) -> Vec<ErrorCategory> {
        vec![
            ErrorCategory::SveltePattern,
            ErrorCategory::SvelteCompiler,
            ErrorCategory::RuntimeSemantic,
        ]
    }

    async fn enrich(&self, error_msg: &str, _category: &ErrorCategory) -> String {
        let mut enriched = error_msg.to_string();

        let rag_guard = self.rag_manager.read().await;
        match rag_guard.search(error_msg, 3).await {
            Ok(docs) if !docs.is_empty() => {
                enriched.push_str("\n\n## Relevant Svelte 5 Documentation:\n");
                for doc in docs {
                    enriched.push_str(&format!(
                        "\n### {} > {}\n{}\n",
                        doc.doc_title, doc.title, doc.content
                    ));
                }
            }
            Ok(_) => {
                log::debug!(
                    "[SvelteDocsEnricher] No relevant docs found for: {}",
                    &error_msg[..error_msg.len().min(100)]
                );
            }
            Err(e) => {
                log::warn!("[SvelteDocsEnricher] Search failed: {}", e);
            }
        }

        enriched
    }
}

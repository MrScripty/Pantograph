//! Error Enricher Pipeline
//!
//! This module provides a pluggable system for enriching validation error messages
//! with relevant documentation and context. Instead of giving the agent tools to
//! search for documentation (which wastes turns), enrichers automatically attach
//! relevant docs to error messages before returning them to the agent.

use async_trait::async_trait;

/// Error category for routing to appropriate enrichers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Svelte 5 syntax pattern errors (export let, on:click, etc.)
    SveltePattern,
    /// Svelte compiler errors (compilation failed)
    SvelteCompiler,
    /// Import resolution errors (package not found)
    ImportResolution,
    /// Runtime semantic errors (type misuse in template)
    RuntimeSemantic,
    /// CSS/styling errors
    Styling,
    /// Non-standard HTML element errors (no docs to fetch)
    HtmlElement,
    /// ESLint code quality errors (no docs to fetch, error message is self-explanatory)
    Linting,
    /// Unknown/other errors
    Other,
}

/// Trait for error enrichers that add context to validation errors
#[async_trait]
pub trait ErrorEnricher: Send + Sync {
    /// Name of this enricher (for logging)
    fn name(&self) -> &'static str;

    /// Which error categories this enricher handles
    fn handles(&self) -> Vec<ErrorCategory>;

    /// Enrich an error message with additional context
    /// Returns the enriched message, or original if enrichment fails/not applicable
    async fn enrich(&self, error_msg: &str, category: &ErrorCategory) -> String;
}

/// Registry of error enrichers
pub struct EnricherRegistry {
    enrichers: Vec<Box<dyn ErrorEnricher>>,
}

impl EnricherRegistry {
    pub fn new() -> Self {
        Self { enrichers: vec![] }
    }

    /// Register an enricher
    pub fn register(&mut self, enricher: Box<dyn ErrorEnricher>) {
        log::info!("[EnricherRegistry] Registered enricher: {}", enricher.name());
        self.enrichers.push(enricher);
    }

    /// Enrich an error message by running all applicable enrichers
    pub async fn enrich(&self, error_msg: &str, category: &ErrorCategory) -> String {
        let mut result = error_msg.to_string();

        for enricher in &self.enrichers {
            if enricher.handles().contains(category) {
                log::debug!(
                    "[EnricherRegistry] Applying enricher '{}' for {:?}",
                    enricher.name(),
                    category
                );
                result = enricher.enrich(&result, category).await;
            }
        }

        result
    }
}

impl Default for EnricherRegistry {
    fn default() -> Self {
        Self::new()
    }
}

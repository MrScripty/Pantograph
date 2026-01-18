pub mod chunker;
pub mod docs;
pub mod docs_index;
pub mod embeddings;
pub mod enricher;
pub mod enricher_svelte;
pub mod prompt;
pub mod rag;
pub mod tools;
pub mod types;

pub use chunker::{preview_chunks, ChunkConfig};
pub use docs::DocsManager;
pub use embeddings::check_embedding_server;
pub use enricher::EnricherRegistry;
pub use enricher_svelte::SvelteDocsEnricher;
pub use prompt::SYSTEM_PROMPT;
pub use rag::{create_rag_manager, SvelteDoc};
pub use tools::*;
pub use types::*;

use crate::config::SandboxConfig;
use rig::client::CompletionClient;
use rig::providers::openai;
use rig::providers::openai::completion::CompletionModel;
use std::path::PathBuf;
use std::sync::Arc;

/// Create an OpenAI-compatible client for the local LLM server
/// Uses the Chat Completions API (/v1/chat/completions) for LM Studio compatibility
pub fn create_client(base_url: &str) -> Result<openai::CompletionsClient, String> {
    // RIG's OpenAI client supports custom base URLs via builder pattern
    // Using "local" as API key since local servers typically don't require one
    // Use completions_api() to get a client that uses /v1/chat/completions
    // Note: RIG expects the base URL to include /v1 suffix for proper endpoint routing
    let base_url_with_v1 = if base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{}/v1", base_url.trim_end_matches('/'))
    };

    openai::Client::builder()
        .api_key("local")
        .base_url(&base_url_with_v1)
        .build()
        .map(|client| client.completions_api())
        .map_err(|e| format!("Failed to create client: {}", e))
}

/// Create the UI generation agent with all tools
///
/// The agent uses an enricher registry to automatically attach relevant documentation
/// to validation errors. Doc search tools are NOT provided to the agent - documentation
/// is served programmatically by the enricher pipeline.
pub fn create_ui_agent(
    client: &openai::CompletionsClient,
    model_name: &str,
    project_root: PathBuf,
    enricher_registry: Arc<EnricherRegistry>,
    write_tracker: WriteTracker,
    sandbox_config: SandboxConfig,
) -> rig::agent::Agent<CompletionModel> {
    let write_tool = WriteGuiFileTool::with_tracker(
        project_root.clone(),
        write_tracker,
        enricher_registry,
    ).with_sandbox_config(sandbox_config);

    // NO doc search tools - documentation is served automatically via the enricher pipeline
    client
        .agent(model_name)
        .preamble(SYSTEM_PROMPT)
        .tool(ReadGuiFileTool::new(project_root.clone()))
        .tool(write_tool)
        .tool(ListComponentsTool::new(project_root.clone()))
        .tool(GetTailwindColorsTool::new())
        .tool(ListTemplatesTool::new(project_root.clone()))
        .tool(ReadTemplateTool::new(project_root))
        .build()
}

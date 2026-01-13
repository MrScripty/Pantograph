pub mod tools;
pub mod prompt;
pub mod types;

pub use tools::*;
pub use prompt::SYSTEM_PROMPT;
pub use types::*;

use rig::client::CompletionClient;
use rig::providers::openai;
use rig::providers::openai::completion::CompletionModel;
use std::path::PathBuf;

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
pub fn create_ui_agent(
    client: &openai::CompletionsClient,
    model_name: &str,
    project_root: PathBuf,
) -> rig::agent::Agent<CompletionModel> {
    client
        .agent(model_name)
        .preamble(SYSTEM_PROMPT)
        .tool(ReadGuiFileTool::new(project_root.clone()))
        .tool(WriteGuiFileTool::new(project_root.clone()))
        .tool(ListComponentsTool::new(project_root.clone()))
        .tool(GetTailwindColorsTool::new())
        .tool(ListTemplatesTool::new(project_root.clone()))
        .tool(ReadTemplateTool::new(project_root))
        .build()
}

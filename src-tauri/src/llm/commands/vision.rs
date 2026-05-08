//! Vision prompt handling commands.

use super::shared::MAX_IMAGE_BASE64_LEN;
use crate::llm::gateway::SharedGateway;
use crate::llm::types::StreamEvent;
use tauri::{command, ipc::Channel, AppHandle, State};

const IMAGE_UNDERSTANDING_CONTRACT_ONLY_ERROR: &str = "Vision prompt execution is disabled until \
image_understanding is implemented through canonical typed inference contracts.";

#[command]
pub async fn send_vision_prompt(
    _app: AppHandle,
    _gateway: State<'_, SharedGateway>,
    _prompt: String,
    image_base64: String,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    // Validate image size to prevent DoS
    if image_base64.len() > MAX_IMAGE_BASE64_LEN {
        return Err(format!(
            "Image too large: {} bytes (max {} bytes)",
            image_base64.len(),
            MAX_IMAGE_BASE64_LEN
        ));
    }

    let error = image_understanding_contract_only_error();
    let _ = channel.send(StreamEvent {
        content: None,
        done: true,
        error: Some(error.clone()),
    });

    Err(error)
}

fn image_understanding_contract_only_error() -> String {
    IMAGE_UNDERSTANDING_CONTRACT_ONLY_ERROR.to_string()
}

#[cfg(test)]
mod tests {
    use super::image_understanding_contract_only_error;

    #[test]
    fn vision_prompt_error_points_to_typed_inference_contracts() {
        let error = image_understanding_contract_only_error();

        assert!(error.contains("image_understanding"));
        assert!(error.contains("canonical typed inference contracts"));
    }
}

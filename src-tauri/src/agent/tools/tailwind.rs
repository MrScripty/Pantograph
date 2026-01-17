use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use super::error::ToolError;
use crate::agent::types::TailwindColors;

// ============================================================================
// GetTailwindColorsTool - Return available Tailwind color palette
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetTailwindColorsArgs {}

#[derive(Clone)]
pub struct GetTailwindColorsTool;

impl GetTailwindColorsTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetTailwindColorsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GetTailwindColorsTool {
    const NAME: &'static str = "get_tailwind_colors";
    type Error = ToolError;
    type Args = GetTailwindColorsArgs;
    type Output = TailwindColors;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the available Tailwind CSS color palette with all color names and their shades".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        use std::collections::HashMap;

        let mut colors = HashMap::new();

        // Standard Tailwind color palette
        let shades = vec![
            "50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"
        ].into_iter().map(String::from).collect::<Vec<_>>();

        for color in &["slate", "gray", "zinc", "neutral", "stone", "red", "orange", "amber",
                       "yellow", "lime", "green", "emerald", "teal", "cyan", "sky", "blue",
                       "indigo", "violet", "purple", "fuchsia", "pink", "rose"] {
            colors.insert(color.to_string(), shades.clone());
        }

        // Special colors
        colors.insert("white".to_string(), vec!["".to_string()]);
        colors.insert("black".to_string(), vec!["".to_string()]);
        colors.insert("transparent".to_string(), vec!["".to_string()]);

        Ok(TailwindColors { colors })
    }
}

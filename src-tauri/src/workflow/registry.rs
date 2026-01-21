//! Node registry - manages available node type definitions
//!
//! The registry stores node definitions for the UI palette.
//! Node execution is handled by node-engine's TaskExecutor.

use std::collections::HashMap;

use super::types::{
    ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

/// Registry of available node types
///
/// Stores node definitions and provides them to the frontend for
/// the node palette. Execution is handled by PantographTaskExecutor.
pub struct NodeRegistry {
    definitions: HashMap<String, NodeDefinition>,
}

impl NodeRegistry {
    /// Create a new registry with all built-in node definitions
    pub fn new() -> Self {
        let mut definitions = HashMap::new();

        // Input nodes
        Self::register(&mut definitions, Self::text_input_definition());
        Self::register(&mut definitions, Self::image_input_definition());
        Self::register(&mut definitions, Self::system_prompt_definition());
        Self::register(&mut definitions, Self::puma_lib_definition());

        // Processing nodes
        Self::register(&mut definitions, Self::llm_inference_definition());
        Self::register(&mut definitions, Self::vision_analysis_definition());
        Self::register(&mut definitions, Self::rag_search_definition());

        // Output nodes
        Self::register(&mut definitions, Self::text_output_definition());
        Self::register(&mut definitions, Self::component_preview_definition());

        // Tool nodes
        Self::register(&mut definitions, Self::agent_tools_definition());
        Self::register(&mut definitions, Self::read_file_definition());
        Self::register(&mut definitions, Self::write_file_definition());

        // Control nodes
        Self::register(&mut definitions, Self::tool_loop_definition());

        Self { definitions }
    }

    /// Register a node definition
    fn register(map: &mut HashMap<String, NodeDefinition>, def: NodeDefinition) {
        map.insert(def.node_type.clone(), def);
    }

    /// Get a node definition by type
    pub fn get_definition(&self, node_type: &str) -> Option<&NodeDefinition> {
        self.definitions.get(node_type)
    }

    /// Get all registered node definitions
    pub fn all_definitions(&self) -> Vec<NodeDefinition> {
        self.definitions.values().cloned().collect()
    }

    /// Get definitions grouped by category
    pub fn definitions_by_category(&self) -> HashMap<String, Vec<NodeDefinition>> {
        let mut grouped: HashMap<String, Vec<NodeDefinition>> = HashMap::new();

        for def in self.definitions.values() {
            let category = format!("{:?}", def.category).to_lowercase();
            grouped
                .entry(category)
                .or_default()
                .push(def.clone());
        }

        grouped
    }

    /// Check if a node type is registered
    pub fn has_node_type(&self, node_type: &str) -> bool {
        self.definitions.contains_key(node_type)
    }

    /// Get the number of registered node types
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    // =========================================================================
    // Node Definition Factories
    // =========================================================================

    fn text_input_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "text-input".to_string(),
            category: NodeCategory::Input,
            label: "Text Input".to_string(),
            description: "Provides text input to the workflow".to_string(),
            inputs: vec![PortDefinition {
                id: "text".to_string(),
                label: "Text".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            outputs: vec![PortDefinition {
                id: "text".to_string(),
                label: "Text".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn image_input_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "image-input".to_string(),
            category: NodeCategory::Input,
            label: "Image Input".to_string(),
            description: "Provides image input to the workflow".to_string(),
            inputs: vec![],
            outputs: vec![PortDefinition {
                id: "image".to_string(),
                label: "Image".to_string(),
                data_type: PortDataType::Image,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn system_prompt_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "system-prompt".to_string(),
            category: NodeCategory::Input,
            label: "System Prompt".to_string(),
            description: "Provides system prompt configuration".to_string(),
            inputs: vec![PortDefinition {
                id: "prompt".to_string(),
                label: "Prompt".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            outputs: vec![PortDefinition {
                id: "prompt".to_string(),
                label: "Prompt".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn puma_lib_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "puma-lib".to_string(),
            category: NodeCategory::Input,
            label: "Puma-Lib".to_string(),
            description: "Provides AI model file path".to_string(),
            inputs: vec![],
            outputs: vec![PortDefinition {
                id: "model_path".to_string(),
                label: "Model Path".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn llm_inference_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "llm-inference".to_string(),
            category: NodeCategory::Processing,
            label: "LLM Inference".to_string(),
            description: "Runs text through a language model".to_string(),
            inputs: vec![
                PortDefinition {
                    id: "prompt".to_string(),
                    label: "Prompt".to_string(),
                    data_type: PortDataType::Prompt,
                    required: true,
                    multiple: false,
                },
                PortDefinition {
                    id: "system_prompt".to_string(),
                    label: "System Prompt".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "model".to_string(),
                    label: "Model".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "image".to_string(),
                    label: "Image".to_string(),
                    data_type: PortDataType::Image,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "audio".to_string(),
                    label: "Audio".to_string(),
                    data_type: PortDataType::Audio,
                    required: false,
                    multiple: false,
                },
            ],
            outputs: vec![
                PortDefinition {
                    id: "response".to_string(),
                    label: "Response".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "stream".to_string(),
                    label: "Stream".to_string(),
                    data_type: PortDataType::Stream,
                    required: false,
                    multiple: false,
                },
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }

    fn vision_analysis_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "vision-analysis".to_string(),
            category: NodeCategory::Processing,
            label: "Vision Analysis".to_string(),
            description: "Analyzes images using a vision model".to_string(),
            inputs: vec![
                PortDefinition {
                    id: "image".to_string(),
                    label: "Image".to_string(),
                    data_type: PortDataType::Image,
                    required: true,
                    multiple: false,
                },
                PortDefinition {
                    id: "prompt".to_string(),
                    label: "Prompt".to_string(),
                    data_type: PortDataType::String,
                    required: true,
                    multiple: false,
                },
            ],
            outputs: vec![PortDefinition {
                id: "analysis".to_string(),
                label: "Analysis".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn rag_search_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "rag-search".to_string(),
            category: NodeCategory::Processing,
            label: "RAG Search".to_string(),
            description: "Searches indexed documents using RAG".to_string(),
            inputs: vec![
                PortDefinition {
                    id: "query".to_string(),
                    label: "Query".to_string(),
                    data_type: PortDataType::String,
                    required: true,
                    multiple: false,
                },
                PortDefinition {
                    id: "limit".to_string(),
                    label: "Limit".to_string(),
                    data_type: PortDataType::Number,
                    required: false,
                    multiple: false,
                },
            ],
            outputs: vec![
                PortDefinition {
                    id: "documents".to_string(),
                    label: "Documents".to_string(),
                    data_type: PortDataType::Json,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "context".to_string(),
                    label: "Context".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn text_output_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "text-output".to_string(),
            category: NodeCategory::Output,
            label: "Text Output".to_string(),
            description: "Displays text output from the workflow".to_string(),
            inputs: vec![PortDefinition {
                id: "text".to_string(),
                label: "Text".to_string(),
                data_type: PortDataType::String,
                required: true,
                multiple: false,
            }],
            outputs: vec![PortDefinition {
                id: "text".to_string(),
                label: "Text".to_string(),
                data_type: PortDataType::String,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn component_preview_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "component-preview".to_string(),
            category: NodeCategory::Output,
            label: "Component Preview".to_string(),
            description: "Previews a Svelte component".to_string(),
            inputs: vec![
                PortDefinition {
                    id: "component".to_string(),
                    label: "Component".to_string(),
                    data_type: PortDataType::Component,
                    required: true,
                    multiple: false,
                },
                PortDefinition {
                    id: "props".to_string(),
                    label: "Props".to_string(),
                    data_type: PortDataType::Json,
                    required: false,
                    multiple: false,
                },
            ],
            outputs: vec![PortDefinition {
                id: "rendered".to_string(),
                label: "Rendered".to_string(),
                data_type: PortDataType::Component,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn agent_tools_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "agent-tools".to_string(),
            category: NodeCategory::Tool,
            label: "Agent Tools".to_string(),
            description: "Configures available tools for agent".to_string(),
            inputs: vec![],
            outputs: vec![PortDefinition {
                id: "tools".to_string(),
                label: "Tools".to_string(),
                data_type: PortDataType::Tools,
                required: false,
                multiple: false,
            }],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn read_file_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "read-file".to_string(),
            category: NodeCategory::Tool,
            label: "Read File".to_string(),
            description: "Reads a file from the filesystem".to_string(),
            inputs: vec![PortDefinition {
                id: "path".to_string(),
                label: "Path".to_string(),
                data_type: PortDataType::String,
                required: true,
                multiple: false,
            }],
            outputs: vec![
                PortDefinition {
                    id: "content".to_string(),
                    label: "Content".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "path".to_string(),
                    label: "Path".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn write_file_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "write-file".to_string(),
            category: NodeCategory::Tool,
            label: "Write File".to_string(),
            description: "Writes content to a file".to_string(),
            inputs: vec![
                PortDefinition {
                    id: "path".to_string(),
                    label: "Path".to_string(),
                    data_type: PortDataType::String,
                    required: true,
                    multiple: false,
                },
                PortDefinition {
                    id: "content".to_string(),
                    label: "Content".to_string(),
                    data_type: PortDataType::String,
                    required: true,
                    multiple: false,
                },
            ],
            outputs: vec![
                PortDefinition {
                    id: "success".to_string(),
                    label: "Success".to_string(),
                    data_type: PortDataType::Boolean,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "path".to_string(),
                    label: "Path".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    fn tool_loop_definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "tool-loop".to_string(),
            category: NodeCategory::Control,
            label: "Tool Loop".to_string(),
            description: "Executes LLM with tools in a loop until completion".to_string(),
            inputs: vec![
                PortDefinition {
                    id: "prompt".to_string(),
                    label: "Prompt".to_string(),
                    data_type: PortDataType::Prompt,
                    required: true,
                    multiple: false,
                },
                PortDefinition {
                    id: "tools".to_string(),
                    label: "Tools".to_string(),
                    data_type: PortDataType::Tools,
                    required: true,
                    multiple: true,
                },
                PortDefinition {
                    id: "max_iterations".to_string(),
                    label: "Max Iterations".to_string(),
                    data_type: PortDataType::Number,
                    required: false,
                    multiple: false,
                },
            ],
            outputs: vec![
                PortDefinition {
                    id: "response".to_string(),
                    label: "Response".to_string(),
                    data_type: PortDataType::String,
                    required: false,
                    multiple: false,
                },
                PortDefinition {
                    id: "tool_calls".to_string(),
                    label: "Tool Calls".to_string(),
                    data_type: PortDataType::Json,
                    required: false,
                    multiple: false,
                },
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_builtin_nodes() {
        let registry = NodeRegistry::new();

        assert!(registry.has_node_type("text-input"));
        assert!(registry.has_node_type("image-input"));
        assert!(registry.has_node_type("system-prompt"));
        assert!(registry.has_node_type("puma-lib"));
        assert!(registry.has_node_type("llm-inference"));
        assert!(registry.has_node_type("vision-analysis"));
        assert!(registry.has_node_type("rag-search"));
        assert!(registry.has_node_type("text-output"));
        assert!(registry.has_node_type("component-preview"));
        assert!(registry.has_node_type("agent-tools"));
        assert!(registry.has_node_type("read-file"));
        assert!(registry.has_node_type("write-file"));
        assert!(registry.has_node_type("tool-loop"));
    }

    #[test]
    fn test_registry_get_definition() {
        let registry = NodeRegistry::new();

        let def = registry.get_definition("text-input").unwrap();
        assert_eq!(def.node_type, "text-input");
        assert_eq!(def.category, NodeCategory::Input);
    }

    #[test]
    fn test_registry_all_definitions() {
        let registry = NodeRegistry::new();

        let all = registry.all_definitions();
        assert!(!all.is_empty());
        assert!(all.len() >= 10); // At least 10 built-in nodes
    }

    #[test]
    fn test_registry_definitions_by_category() {
        let registry = NodeRegistry::new();

        let grouped = registry.definitions_by_category();

        assert!(grouped.contains_key("input"));
        assert!(grouped.contains_key("processing"));
        assert!(grouped.contains_key("output"));
        assert!(grouped.contains_key("tool"));
        assert!(grouped.contains_key("control"));
    }
}

use pantograph_workflow_service::{
    validate_workflow_connection as validate_connection_internal, NodeDefinition, NodeRegistry,
    PortDataType,
};

pub fn validate_workflow_connection(source_type: PortDataType, target_type: PortDataType) -> bool {
    validate_connection_internal(&source_type, &target_type)
}

pub fn get_node_definitions() -> Vec<NodeDefinition> {
    NodeRegistry::new().all_definitions()
}

pub fn get_node_definitions_by_category() -> std::collections::HashMap<String, Vec<NodeDefinition>>
{
    NodeRegistry::new().definitions_by_category()
}

pub fn get_node_definition(node_type: String) -> Option<NodeDefinition> {
    NodeRegistry::new().get_definition(&node_type).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_connection() {
        assert!(validate_workflow_connection(
            PortDataType::String,
            PortDataType::String
        ));
        assert!(validate_workflow_connection(
            PortDataType::String,
            PortDataType::Prompt
        ));
        assert!(validate_workflow_connection(
            PortDataType::Any,
            PortDataType::Image
        ));
        assert!(!validate_workflow_connection(
            PortDataType::Image,
            PortDataType::String
        ));
    }

    #[test]
    fn test_get_node_definitions() {
        let defs = get_node_definitions();
        assert!(!defs.is_empty());

        assert!(defs.iter().any(|d| d.node_type == "text-input"));
        assert!(defs.iter().any(|d| d.node_type == "llm-inference"));
        assert!(defs.iter().any(|d| d.node_type == "text-output"));
    }

    #[test]
    fn test_get_node_definitions_by_category() {
        let grouped = get_node_definitions_by_category();

        assert!(grouped.contains_key("input"));
        assert!(grouped.contains_key("processing"));
        assert!(grouped.contains_key("output"));
    }

    #[test]
    fn test_get_node_definition() {
        let def = get_node_definition("text-input".to_string());
        assert!(def.is_some());
        assert_eq!(
            def.expect("text-input should exist").node_type,
            "text-input"
        );

        let missing = get_node_definition("nonexistent".to_string());
        assert!(missing.is_none());
    }
}

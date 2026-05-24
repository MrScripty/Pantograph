use pantograph_node_contracts::NodeTypeContract;

use super::task_graph_contracts::WorkflowSchedulerTaskExecutionClass;

const NODE_TYPE_LLM_INFERENCE: &str = "llm-inference";
const NODE_TYPE_PUMA_LIB: &str = "puma-lib";
const NODE_TYPE_BOOLEAN_INPUT: &str = "boolean-input";
const NODE_TYPE_TEXT_INPUT: &str = "text-input";
const NODE_TYPE_TEXT_OUTPUT: &str = "text-output";

pub(super) fn classify_workflow_scheduler_task(
    node_type: &str,
    contract: Option<&NodeTypeContract>,
) -> WorkflowSchedulerTaskExecutionClass {
    if node_type == NODE_TYPE_PUMA_LIB {
        return WorkflowSchedulerTaskExecutionClass::PumasMaterialization;
    }

    let Some(contract) = contract else {
        return WorkflowSchedulerTaskExecutionClass::Unsupported;
    };

    if contract.node_type.as_str() != node_type {
        return WorkflowSchedulerTaskExecutionClass::Unsupported;
    }

    if node_type == NODE_TYPE_LLM_INFERENCE && !contract.inference_tasks.is_empty() {
        return WorkflowSchedulerTaskExecutionClass::RuntimeInference;
    }

    if !contract.inference_tasks.is_empty() {
        return WorkflowSchedulerTaskExecutionClass::Unsupported;
    }

    if is_first_stage_node_engine_task(node_type) {
        return WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine;
    }

    WorkflowSchedulerTaskExecutionClass::Unsupported
}

fn is_first_stage_node_engine_task(node_type: &str) -> bool {
    matches!(
        node_type,
        NODE_TYPE_BOOLEAN_INPUT | NODE_TYPE_TEXT_INPUT | NODE_TYPE_TEXT_OUTPUT
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(node_type: &str) -> NodeTypeContract {
        workflow_nodes::builtin_node_contracts()
            .expect("built-in node contracts")
            .into_iter()
            .find(|contract| contract.node_type.as_str() == node_type)
            .unwrap_or_else(|| panic!("missing contract for {node_type}"))
    }

    #[test]
    fn classifier_marks_llm_inference_as_runtime_inference() {
        let contract = contract("llm-inference");

        assert_eq!(
            classify_workflow_scheduler_task("llm-inference", Some(&contract)),
            WorkflowSchedulerTaskExecutionClass::RuntimeInference
        );
    }

    #[test]
    fn classifier_marks_first_stage_scalar_nodes_as_non_runtime_node_engine() {
        for node_type in ["boolean-input", "text-input", "text-output"] {
            let contract = contract(node_type);

            assert_eq!(
                classify_workflow_scheduler_task(node_type, Some(&contract)),
                WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
                "{node_type} should be first-stage non-runtime node-engine"
            );
        }
    }

    #[test]
    fn classifier_marks_puma_lib_as_materialization_boundary() {
        let contract = contract("puma-lib");

        assert_eq!(
            classify_workflow_scheduler_task("puma-lib", Some(&contract)),
            WorkflowSchedulerTaskExecutionClass::PumasMaterialization
        );
    }

    #[test]
    fn classifier_rejects_excluded_and_unknown_nodes() {
        for node_type in ["model-provider", "expand-settings", "image-output"] {
            let contract = contract(node_type);

            assert_eq!(
                classify_workflow_scheduler_task(node_type, Some(&contract)),
                WorkflowSchedulerTaskExecutionClass::Unsupported,
                "{node_type} should not enter the first-stage adapter"
            );
        }

        assert_eq!(
            classify_workflow_scheduler_task("not-registered", None),
            WorkflowSchedulerTaskExecutionClass::Unsupported
        );
    }

    #[test]
    fn classifier_requires_matching_contract_facts() {
        let text_contract = contract("text-input");
        let inference_contract = contract("llm-inference");

        assert_eq!(
            classify_workflow_scheduler_task("llm-inference", Some(&text_contract)),
            WorkflowSchedulerTaskExecutionClass::Unsupported
        );
        assert_eq!(
            classify_workflow_scheduler_task("text-input", Some(&inference_contract)),
            WorkflowSchedulerTaskExecutionClass::Unsupported
        );
    }
}

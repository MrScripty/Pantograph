use std::collections::HashMap;

#[cfg(feature = "inference-nodes")]
use super::read_optional_input_value_aliases;
#[cfg(feature = "inference-nodes")]
use crate::error::{NodeEngineError, Result};

mod input_projection;
pub(crate) use input_projection::*;
mod planning_projection;
#[allow(unused_imports)]
pub(crate) use planning_projection::*;

#[cfg(feature = "inference-nodes")]
pub(crate) fn reject_retired_model_reference_inputs(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    reject_retired_resolved_model_source_inputs(inputs)?;
    reject_unresolved_model_reference_inputs(inputs)?;
    Ok(())
}

#[cfg(feature = "inference-nodes")]
fn reject_retired_resolved_model_source_inputs(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    if read_optional_input_value_aliases(inputs, &["resolved_model_source", "resolvedModelSource"])
        .is_some()
    {
        return Err(NodeEngineError::ExecutionFailed(
            "Retired resolved_model_source input cannot provide executable model paths. Use canonical pumas_model_ref and host-provided planning facts instead."
                .to_string(),
        ));
    }

    Ok(())
}

#[cfg(feature = "inference-nodes")]
fn reject_unresolved_model_reference_inputs(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    for (field_name, aliases) in [("pumas_model_ref", ["pumas_model_ref", "pumasModelRef"])] {
        let Some(raw) = read_optional_input_value_aliases(inputs, &aliases) else {
            continue;
        };
        if !model_reference_status_is_unresolved(&raw) {
            continue;
        }
        let source = raw
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Canonical inference model reference is unresolved in {field_name} from {source}. Resolve this model through Pumas before execution."
        )));
    }

    Ok(())
}

#[cfg(feature = "inference-nodes")]
fn model_reference_status_is_unresolved(value: &serde_json::Value) -> bool {
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("unresolved"))
}

// ---------------------------------------------------------------------------
// Retired direct-backend inference nodes
// ---------------------------------------------------------------------------

//! Compatibility projection without execution-cost inputs.

use pantograph_dependency_planning::{
    DependencyTaskId, DeviceIntentId, PumasModelRef, RuntimeIntentId,
};

use super::bounded_grouping::{
    count, preflight_model_ref, RawStringBudget, MAX_ID_BYTES, MAX_SHAPE_BYTES,
};
use super::{
    validate_input_shape_signature, SchedulerBoundedGroupingIncompatibility,
    SchedulerBoundedGroupingRefusal, SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS,
};

/// Borrowed caller-owned facts for a pure planning compatibility comparison.
///
/// These fields do not attest resolved model/runtime/device authority, Ready
/// state, ownership, service opportunities, or physical resource capacity. A
/// shape label is not proof of actual tensor compatibility or native batching.
/// No latency or memory estimate is required or inferred by this projection.
#[derive(Clone, Copy)]
pub struct SchedulerGroupingCompatibility<'a> {
    pub task_family: &'a DependencyTaskId,
    pub model_ref: &'a PumasModelRef,
    pub runtime_id: &'a RuntimeIntentId,
    pub device_ids: &'a [DeviceIntentId],
    pub input_shape_signature: &'a str,
}

/// Return the unchanged compatible window length, or one for incompatibility.
///
/// The window must contain one through eight views. Every nested count and raw
/// string length, including the aggregate 64 KiB string budget, is checked for
/// all views before any model validation or equality comparison. Invalid input
/// refuses the entire window before compatibility is examined. Models compare
/// in full, devices retain their order, and no input is retained or mutated.
///
/// This is a planning projection, not the full batch-candidate contract: it does
/// not validate member identity/correlation or produce a memory sum. The existing
/// bounded grouping API continues to require those independent contracts.
pub fn evaluate_scheduler_bounded_compatibility(
    candidates: &[SchedulerGroupingCompatibility<'_>],
) -> Result<usize, SchedulerBoundedGroupingRefusal> {
    if candidates.is_empty() || candidates.len() > SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS {
        return Err(SchedulerBoundedGroupingRefusal::InvalidWindow {
            candidate_count: candidates.len(),
        });
    }
    let mut budget = RawStringBudget::default();
    for (index, candidate) in candidates.iter().enumerate() {
        count(index, "device_ids", candidate.device_ids.len())?;
        budget.text(
            index,
            "task_family",
            candidate.task_family.as_str(),
            MAX_ID_BYTES,
        )?;
        preflight_model_ref(candidate.model_ref, index, "model_ref", &mut budget)?;
        budget.text(
            index,
            "runtime_id",
            candidate.runtime_id.as_str(),
            MAX_ID_BYTES,
        )?;
        for device in candidate.device_ids {
            budget.text(index, "device_ids", device.as_str(), MAX_ID_BYTES)?;
        }
        budget.text(
            index,
            "input_shape_signature",
            candidate.input_shape_signature,
            MAX_SHAPE_BYTES,
        )?;
    }
    for (index, candidate) in candidates.iter().enumerate() {
        let invalid = || SchedulerBoundedGroupingRefusal::InvalidCandidate {
            candidate_index: index,
        };
        candidate.model_ref.validate().map_err(|_| invalid())?;
        validate_input_shape_signature(candidate.input_shape_signature).map_err(|_| invalid())?;
        if candidate.device_ids.is_empty()
            || candidate
                .device_ids
                .iter()
                .enumerate()
                .any(|(position, device)| candidate.device_ids[..position].contains(device))
        {
            return Err(invalid());
        }
    }
    let first = candidates[0];
    if candidates[1..]
        .iter()
        .any(|candidate| compatibility_incompatibility(first, *candidate).is_some())
    {
        Ok(1)
    } else {
        Ok(candidates.len())
    }
}

pub(super) fn compatibility_incompatibility(
    first: SchedulerGroupingCompatibility<'_>,
    candidate: SchedulerGroupingCompatibility<'_>,
) -> Option<SchedulerBoundedGroupingIncompatibility> {
    if candidate.task_family != first.task_family {
        Some(SchedulerBoundedGroupingIncompatibility::TaskFamily)
    } else if candidate.model_ref != first.model_ref {
        Some(SchedulerBoundedGroupingIncompatibility::ModelReference)
    } else if candidate.runtime_id != first.runtime_id {
        Some(SchedulerBoundedGroupingIncompatibility::Runtime)
    } else if candidate.device_ids != first.device_ids {
        Some(SchedulerBoundedGroupingIncompatibility::DeviceSet)
    } else if candidate.input_shape_signature != first.input_shape_signature {
        Some(SchedulerBoundedGroupingIncompatibility::InputShape)
    } else {
        None
    }
}

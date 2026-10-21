//! Finite, immediate grouping proposals over one caller-owned candidate window.

use pantograph_dependency_planning::PumasModelRef;

use crate::intent::SchedulerTraitValue;

use super::{batch_candidate_incompatibility, SchedulerBatchCandidate};

/// Largest candidate window accepted by the bounded grouping policy.
pub const SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS: usize = 8;
/// Maximum aggregate raw-string bytes admitted before validation or comparison.
///
/// This bounds input size, not total byte visits: validators and compatibility
/// comparisons make a finite number of additional passes over admitted strings.
pub const SCHEDULER_BOUNDED_GROUPING_MAX_RAW_STRING_BYTES: usize = 64 * 1024;

const MAX_NESTED_ITEMS: usize = 8;
const MAX_ID_BYTES: usize = 128;
const MAX_MODEL_ID_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 1024;
const MAX_SHAPE_BYTES: usize = 128;

/// First compatibility field that prevents grouping with the anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerBoundedGroupingIncompatibility {
    TaskFamily,
    ModelReference,
    Runtime,
    DeviceSet,
    InputShape,
}

/// Why the unchanged input prefix was proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerBoundedGroupingDisposition {
    Singleton,
    CompatibleGroup,
    Incompatible(SchedulerBoundedGroupingIncompatibility),
}

/// An immutable advisory prefix and its checked incremental memory sum.
///
/// This value does not bind identities or attest Ready state, CPU capability,
/// current ownership, physical capacity, or a service opportunity. A consumer
/// must revalidate its authoritative snapshot before each member's normal Start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SchedulerBoundedGroupingProposal {
    selected_count: usize,
    total_incremental_memory_bytes: u64,
    disposition: SchedulerBoundedGroupingDisposition,
}

impl SchedulerBoundedGroupingProposal {
    /// Selects exactly the original input prefix `[0..selected_count)`.
    #[must_use]
    pub fn selected_count(&self) -> usize {
        self.selected_count
    }

    /// Checked sum for selected members only; this is not a resource reservation.
    #[must_use]
    pub fn total_incremental_memory_bytes(&self) -> u64 {
        self.total_incremental_memory_bytes
    }

    #[must_use]
    pub fn disposition(&self) -> SchedulerBoundedGroupingDisposition {
        self.disposition
    }
}

/// Fixed, non-sensitive refusal context. Candidate contents are never retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SchedulerBoundedGroupingRefusal {
    #[error("bounded grouping requires one through eight candidates; received {candidate_count}")]
    InvalidWindow { candidate_count: usize },
    #[error("bounded grouping input limit at candidate {candidate_index}, field {field}")]
    InputLimit {
        candidate_index: usize,
        field: &'static str,
    },
    #[error("bounded grouping aggregate raw-string byte limit exceeded")]
    RawStringBudgetExceeded,
    #[error("bounded grouping candidate {candidate_index} fails the existing candidate contract")]
    InvalidCandidate { candidate_index: usize },
    #[error("bounded grouping candidates {first_index} and {duplicate_index} repeat a run/task identity")]
    DuplicateMember {
        first_index: usize,
        duplicate_index: usize,
    },
    #[error("bounded grouping selected-member incremental memory sum overflows")]
    MemoryOverflow,
}

/// Propose an immediate bounded prefix, preserving candidate order.
///
/// All raw nested counts and byte lengths are preflighted for every member before
/// any existing candidate validator or compatibility comparison runs. Invalid or
/// duplicate members refuse the whole window before incompatibility is examined.
/// An otherwise valid incompatible window selects only its first member; a fully
/// compatible window selects all members. No fill timer or queue scan is added.
///
/// This pure operation grants nothing and changes no scheduler, queue, resource,
/// or default-policy state. It does not validate upstream dependency correctness
/// or authorize combining several members under a single M3 service opportunity.
pub fn evaluate_scheduler_bounded_grouping(
    candidates: &[SchedulerBatchCandidate],
) -> Result<SchedulerBoundedGroupingProposal, SchedulerBoundedGroupingRefusal> {
    if candidates.is_empty() || candidates.len() > SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS {
        return Err(SchedulerBoundedGroupingRefusal::InvalidWindow {
            candidate_count: candidates.len(),
        });
    }

    let mut budget = RawStringBudget::default();
    for (index, candidate) in candidates.iter().enumerate() {
        preflight_candidate(candidate, index, &mut budget)?;
    }
    for (index, candidate) in candidates.iter().enumerate() {
        candidate
            .validate()
            .map_err(|_| SchedulerBoundedGroupingRefusal::InvalidCandidate {
                candidate_index: index,
            })?;
    }
    for (index, candidate) in candidates.iter().enumerate() {
        if let Some(first_index) = candidates[..index].iter().position(|earlier| {
            earlier.workflow_run_id == candidate.workflow_run_id
                && earlier.task_id == candidate.task_id
        }) {
            return Err(SchedulerBoundedGroupingRefusal::DuplicateMember {
                first_index,
                duplicate_index: index,
            });
        }
    }

    let first = &candidates[0];
    for candidate in &candidates[1..] {
        if let Some(reason) = batch_candidate_incompatibility(first, candidate) {
            return Ok(SchedulerBoundedGroupingProposal {
                selected_count: 1,
                total_incremental_memory_bytes: first.memory_impact.incremental_bytes,
                disposition: SchedulerBoundedGroupingDisposition::Incompatible(reason),
            });
        }
    }
    let total_incremental_memory_bytes =
        candidates.iter().try_fold(0_u64, |total, candidate| {
            total
                .checked_add(candidate.memory_impact.incremental_bytes)
                .ok_or(SchedulerBoundedGroupingRefusal::MemoryOverflow)
        })?;
    Ok(SchedulerBoundedGroupingProposal {
        selected_count: candidates.len(),
        total_incremental_memory_bytes,
        disposition: if candidates.len() == 1 {
            SchedulerBoundedGroupingDisposition::Singleton
        } else {
            SchedulerBoundedGroupingDisposition::CompatibleGroup
        },
    })
}

#[derive(Default)]
struct RawStringBudget {
    admitted_bytes: usize,
}

impl RawStringBudget {
    fn text(
        &mut self,
        candidate_index: usize,
        field: &'static str,
        value: &str,
        maximum: usize,
    ) -> Result<(), SchedulerBoundedGroupingRefusal> {
        if value.len() > maximum {
            return Err(SchedulerBoundedGroupingRefusal::InputLimit {
                candidate_index,
                field,
            });
        }
        self.admitted_bytes = self
            .admitted_bytes
            .checked_add(value.len())
            .filter(|total| *total <= SCHEDULER_BOUNDED_GROUPING_MAX_RAW_STRING_BYTES)
            .ok_or(SchedulerBoundedGroupingRefusal::RawStringBudgetExceeded)?;
        Ok(())
    }

    fn optional_text(
        &mut self,
        index: usize,
        field: &'static str,
        value: Option<&str>,
        maximum: usize,
    ) -> Result<(), SchedulerBoundedGroupingRefusal> {
        if let Some(value) = value {
            self.text(index, field, value, maximum)?;
        }
        Ok(())
    }
}

fn count(
    index: usize,
    field: &'static str,
    length: usize,
) -> Result<(), SchedulerBoundedGroupingRefusal> {
    if length > MAX_NESTED_ITEMS {
        return Err(SchedulerBoundedGroupingRefusal::InputLimit {
            candidate_index: index,
            field,
        });
    }
    Ok(())
}

fn preflight_model_ref(
    model: &PumasModelRef,
    index: usize,
    field: &'static str,
    budget: &mut RawStringBudget,
) -> Result<(), SchedulerBoundedGroupingRefusal> {
    count(index, field, model.migration_diagnostics.len())?;
    budget.text(index, field, &model.model_id, MAX_MODEL_ID_BYTES)?;
    budget.optional_text(index, field, model.revision.as_deref(), MAX_MODEL_ID_BYTES)?;
    budget.optional_text(
        index,
        field,
        model.selected_artifact_id.as_deref(),
        MAX_MODEL_ID_BYTES,
    )?;
    budget.optional_text(
        index,
        field,
        model.selected_artifact_path.as_deref(),
        MAX_TEXT_BYTES,
    )?;
    for diagnostic in &model.migration_diagnostics {
        budget.text(index, field, &diagnostic.code, MAX_ID_BYTES)?;
        budget.text(index, field, &diagnostic.message, MAX_TEXT_BYTES)?;
        budget.optional_text(index, field, diagnostic.input.as_deref(), MAX_TEXT_BYTES)?;
    }
    Ok(())
}

fn preflight_candidate(
    candidate: &SchedulerBatchCandidate,
    index: usize,
    budget: &mut RawStringBudget,
) -> Result<(), SchedulerBoundedGroupingRefusal> {
    let intent = &candidate.task_intent;
    count(
        index,
        "selected_device_ids",
        candidate.selected_device_ids.len(),
    )?;
    count(index, "trait_settings", intent.trait_settings.len())?;
    count(
        index,
        "dependency_override_patches",
        intent.dependency_override_patches.len(),
    )?;
    count(index, "estimate_hints", intent.estimate_hints.len())?;
    preflight_model_ref(
        &candidate.selected_model_ref,
        index,
        "selected_model_ref",
        budget,
    )?;
    preflight_model_ref(&intent.model_ref, index, "task_intent.model_ref", budget)?;
    for value in [
        candidate.workflow_id.as_str(),
        candidate.workflow_run_id.as_str(),
        candidate.node_id.as_str(),
        candidate.task_id.as_str(),
        candidate.task_family.as_str(),
        candidate.selected_runtime_id.as_str(),
        intent.workflow_id.as_str(),
        intent.workflow_run_id.as_str(),
        intent.node_id.as_str(),
        intent.task_id.as_str(),
        intent.task_type.as_str(),
    ] {
        budget.text(index, "identity", value, MAX_ID_BYTES)?;
    }
    for device in &candidate.selected_device_ids {
        budget.text(index, "selected_device_ids", device.as_str(), MAX_ID_BYTES)?;
    }
    budget.optional_text(
        index,
        "fairness_key",
        intent.fairness_key.as_ref().map(|key| key.as_str()),
        MAX_ID_BYTES,
    )?;
    budget.optional_text(
        index,
        "requested_runtime_id",
        intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .map(|id| id.as_str()),
        MAX_ID_BYTES,
    )?;
    budget.optional_text(
        index,
        "requested_device_id",
        intent
            .constraints
            .requested_device_id
            .as_ref()
            .map(|id| id.as_str()),
        MAX_ID_BYTES,
    )?;
    budget.text(
        index,
        "input_shape_signature",
        &candidate.input_shape_signature,
        MAX_SHAPE_BYTES,
    )?;
    for setting in &intent.trait_settings {
        budget.text(
            index,
            "trait_settings",
            setting.trait_id.as_str(),
            MAX_ID_BYTES,
        )?;
        if let SchedulerTraitValue::String(value) = &setting.value {
            budget.text(index, "trait_settings", value, MAX_TEXT_BYTES)?;
        }
    }
    for patch in &intent.dependency_override_patches {
        let fields = &patch.fields;
        if let Some(urls) = &fields.extra_index_urls {
            count(index, "dependency_override.extra_index_urls", urls.len())?;
        }
        budget.text(
            index,
            "dependency_override",
            &patch.binding_id,
            MAX_TEXT_BYTES,
        )?;
        for value in [
            patch.requirement_name.as_deref(),
            patch.source.as_deref(),
            patch.updated_at.as_deref(),
            fields.python_executable.as_deref(),
            fields.index_url.as_deref(),
            fields.wheel_source_path.as_deref(),
            fields.package_source_override.as_deref(),
        ] {
            budget.optional_text(index, "dependency_override", value, MAX_TEXT_BYTES)?;
        }
        if let Some(urls) = &fields.extra_index_urls {
            for value in urls {
                budget.text(
                    index,
                    "dependency_override.extra_index_urls",
                    value,
                    MAX_TEXT_BYTES,
                )?;
            }
        }
    }
    Ok(())
}

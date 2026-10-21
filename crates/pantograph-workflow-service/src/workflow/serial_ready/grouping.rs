//! Pure derived keys only. No proposal, owner, reservation or service turn is
//! retained here; each member follows the ordinary authoritative worker path.

use super::{SerialReadyMode, WorkflowSerialReadyMember};
use pantograph_runtime_host_contracts::RuntimeHostExecutionInput;
use pantograph_scheduler::{
    evaluate_scheduler_bounded_compatibility, SchedulerGroupingCompatibility,
    SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS,
};

#[derive(Default)]
pub(super) struct WorkloadKeyGroup {
    entries: Vec<(WorkflowSerialReadyMember, [u8; 32])>,
}

impl WorkloadKeyGroup {
    fn key(&self, member: &WorkflowSerialReadyMember) -> Option<[u8; 32]> {
        self.entries
            .iter()
            .find_map(|(snapshot, key)| (snapshot == member).then_some(*key))
    }
}

impl SerialReadyMode {
    pub(super) fn finish_grouping_member(&self, member: Option<&WorkflowSerialReadyMember>) {
        if !self.config.bounded_grouping {
            return;
        }
        if let Ok(mut group) = self.grouping.lock() {
            if let Some(member) = member {
                group.entries.retain(|(snapshot, _)| {
                    snapshot.task.workflow_run_id != member.task.workflow_run_id
                        || snapshot.task.task_id != member.task.task_id
                });
            } else {
                group.entries.clear();
            }
        }
    }

    pub(crate) fn grouping_needs_refresh(&self, anchor: &WorkflowSerialReadyMember) -> bool {
        self.config.bounded_grouping
            && (!anchor.grouping_bounded()
                || self
                    .grouping
                    .lock()
                    .map_or(true, |group| group.key(anchor).is_none()))
    }

    pub(crate) fn refresh_grouping(&self, window: Option<Vec<WorkflowSerialReadyMember>>) {
        // Compute outside the cache lock. The store guard and all native owner
        // queries have already ended; this operation has no callbacks or awaits.
        let entries = window.and_then(|members| {
            if members.is_empty()
                || members.len() > SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS
                || members.iter().any(|member| !member.grouping_bounded())
            {
                return None;
            }
            let views = members
                .iter()
                .map(|member| {
                    let intent = member.task.schedulable_intent.as_ref()?;
                    Some(SchedulerGroupingCompatibility {
                        task_family: &intent.task_type,
                        model_ref: &intent.model_ref,
                        runtime_id: intent.constraints.requested_runtime_id.as_ref()?,
                        device_ids: std::slice::from_ref(
                            intent.constraints.requested_device_id.as_ref()?,
                        ),
                        // A known input kind, not a tokenizer/tensor dimension.
                        input_shape_signature: "single_utf8_text",
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            let count = evaluate_scheduler_bounded_compatibility(&views).ok()?;
            let mut entries = Vec::with_capacity(count);
            for member in members.into_iter().take(count) {
                let key = self.compute_key(&member, &member.inputs)?;
                entries.push((member, key));
            }
            Some(entries)
        });
        if let Ok(mut cache) = self.grouping.lock() {
            cache.entries = entries.unwrap_or_default();
        }
    }

    pub(super) fn key_for(&self, member: &WorkflowSerialReadyMember) -> Option<[u8; 32]> {
        // Bound the complete current equality operand before comparing it with
        // stored snapshots. A miss never upgrades or changes ordinary behavior.
        if self.config.bounded_grouping && member.grouping_bounded() {
            let key = self
                .grouping
                .lock()
                .ok()
                .and_then(|group| group.key(member));
            if let Some(key) = key {
                #[cfg(any(test, feature = "test-support"))]
                self.key_cache_hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(key);
            }
        }
        self.compute_key(member, &member.inputs)
    }

    pub(super) fn compute_key(
        &self,
        member: &WorkflowSerialReadyMember,
        inputs: &[RuntimeHostExecutionInput],
    ) -> Option<[u8; 32]> {
        super::observation_key_with_inputs(member, inputs, || {
            // Count every actual key encoding, including eager fill, misses,
            // lookahead and the independent final native-request input fence.
            #[cfg(any(test, feature = "test-support"))]
            self.key_encodings
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        })
    }
}

impl super::WorkflowService {
    /// Test-only mechanism counts, including independent final-request hashing.
    /// These counters establish no end-to-end or real-model performance result.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_grouping_work_for_test(&self) -> (u64, u64, usize) {
        use std::sync::atomic::Ordering;
        self.serial_ready_mode.as_ref().map_or((0, 0, 0), |mode| {
            (
                mode.key_encodings.load(Ordering::Relaxed),
                mode.key_cache_hits.load(Ordering::Relaxed),
                mode.grouping.lock().map_or(0, |group| group.entries.len()),
            )
        })
    }
}

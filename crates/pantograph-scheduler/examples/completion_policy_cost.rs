//! Synthetic policy-only dispatch cost. No runtime, models or leases.
//! Run with cargo run --release -p pantograph-scheduler --example completion_policy_cost.
use std::hint::black_box;
use std::time::Instant;

use pantograph_scheduler::*;

fn main() {
    let fixture: SchedulerDispatchSelectionRequest = serde_json::from_str(include_str!(
        "../tests/fixtures/dispatch_selection_request_valid.json"
    ))
    .unwrap();
    let policy = SchedulerCompletionRankingPolicy {
        now_ms: 1000,
        max_sample_age_ms: 100,
        minimum_samples: 3,
        allow_synthetic: true,
    };
    let context = SchedulerCompletionContext {
        host_id: "synthetic-host",
        runtime_instance_id: "synthetic-instance",
        artifact_fingerprint: "synthetic-artifact",
        workload_fingerprint: "synthetic-workload",
        resource_condition_fingerprint: "synthetic-idle",
        residency_fingerprint: "synthetic-cold",
        timing_convention: "synthetic-serialized-single-task-mean-us-v1",
    };
    println!("source=synthetic mode=policy-only units=nanoseconds samples=10000");
    let long_context_text = "x".repeat(128);
    for (name, size, maximum_fields, fallback) in [
        ("typical", 1, false, false),
        ("typical", 8, false, false),
        ("typical", SCHEDULER_COMPLETION_MAX_CANDIDATES, false, false),
        (
            "maximum-fields",
            SCHEDULER_COMPLETION_MAX_CANDIDATES,
            true,
            false,
        ),
        (
            "last-row-invalid",
            SCHEDULER_COMPLETION_MAX_CANDIDATES,
            true,
            true,
        ),
    ] {
        let context = if maximum_fields {
            SchedulerCompletionContext {
                host_id: &long_context_text,
                runtime_instance_id: &long_context_text,
                artifact_fingerprint: &long_context_text,
                workload_fingerprint: &long_context_text,
                resource_condition_fingerprint: &long_context_text,
                residency_fingerprint: &long_context_text,
                timing_convention: &long_context_text,
            }
        } else {
            context
        };
        let mut request = fixture.clone();
        if maximum_fields {
            let mut diagnostic = fixture.candidates[0].candidate_source_diagnostics[0].clone();
            diagnostic.message = "x".repeat(1024);
            diagnostic.hint = Some("x".repeat(1024));
            request.diagnostics = vec![diagnostic; 32];
        }
        request.candidates = (0..size)
            .map(|index| {
                let mut candidate = fixture.candidates[0].clone();
                let mut id = format!("candidate.{index:03}");
                if maximum_fields {
                    id.push_str(&"x".repeat(128 - id.len()));
                }
                candidate.candidate_id = id.parse().unwrap();
                candidate.reservations.clear();
                candidate.batching_group_id = None;
                candidate
            })
            .collect();
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(request).unwrap();
        let mut evidence: Vec<_> = request
            .as_ref()
            .candidates
            .iter()
            .map(|candidate| SchedulerCompletionEvidence {
                request: &request,
                candidate,
                current_context: context,
                sample: SchedulerCompletionSample {
                    candidate,
                    context,
                    source: SchedulerCompletionEvidenceSource::Synthetic,
                    sample_count: 3,
                    observed_at_ms: 950,
                    preparation_us: Some(1000),
                    required_transfer_us: Some(1000),
                    execution_us: Some(1000),
                },
            })
            .collect();
        if fallback {
            evidence.last_mut().unwrap().sample.execution_us = None;
        }
        let result = select_scheduler_candidate_with_completion(&request, &evidence, policy);
        assert_eq!(
            matches!(
                result.diagnostic,
                SchedulerCompletionRankingDiagnostic::Fallback(_)
            ),
            fallback
        );
        for _ in 0..1000 {
            black_box(select_scheduler_candidate_with_completion(
                &request, &evidence, policy,
            ));
        }
        let mut samples = Vec::with_capacity(10000);
        let started = Instant::now();
        for _ in 0..10000 {
            let call_start = Instant::now();
            black_box(select_scheduler_candidate_with_completion(
                black_box(&request),
                black_box(&evidence),
                policy,
            ));
            samples.push(call_start.elapsed().as_nanos());
        }
        let wall = started.elapsed();
        samples.sort_unstable();
        println!(
            "fixture={name} candidates={size} p50={} p95={} max={} loop_wall_ms={:.3}",
            samples[5000],
            samples[9500],
            samples[9999],
            wall.as_secs_f64() * 1000.0
        );
    }
}

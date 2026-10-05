# Scheduler choice before resource commitment

Status: source candidate for parent-owned native execution and review. No PR,
main merge or external review request.

## Source and admitted scope

Branch `feat/scheduler-selected-reservation` starts at reservation evaluation
`1f6401b7282cb88404c5174115a5b2cb5f655f57`, tree
`1d5cb3957e6f5e7a937d34bb158946f388061293`. Its ordered feature ancestry retains
image sink `3fd46ff4a21d881b618fe84a64961036d1476d9f`, JSON filter and text merge,
then PR52 and PR44. It does not contain the repaired-controls composition;
parent coordinates subsequent composition. Frozen refs are unchanged. Pumas
remains pinned to `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`; pooling candidate
`495efea606620153fe2f0363b104e56518436a06` remains excluded.

The coordinator authorized implementation of production prerequisites needed
for ranking. This slice owns evaluate -> scheduler choice -> commit for current
explicit runtime/device candidates. Acceptance: rejected, duplicate and ambiguous
alternatives acquire no leases; the sole eligible candidate acquires exactly one;
an advisory choice cannot authorize execution. Product policy remains in the
scheduler; the embedded provider supplies real registry observations and commits
the selected candidate. No learned ranking, hardware calibration or shared
physical-memory accounting is claimed.

## Behavior and ownership

Previously the provider acquired real leases while discovering each alternative
and used a fabricated private lease for preliminary identity checks. It now
checks immutable identity without a lease, evaluates each real registry request
without mutation, refreshes runtime status/instance from that observation, and
passes unreserved candidates to a scheduler-owned policy function. Duplicate IDs
and multiple eligible candidates fail closed. The policy retains explicit runtime
and device constraints and requires resource-fit evidence.

The new in-process reservation selection carries an ID and diagnostics, not an
executable dispatch. Existing dispatch validation still rejects missing leases.
Only the selected ID is committed through fresh registry evaluation and atomic
acquisition; contention or changed admission can still reject commitment. Existing
reserved evidence and workflow fact validation remain in place. No public wire
schema, dependency, network setting or audit/traceability gate was changed.

The observation remains advisory: this does not introduce a global physical
memory domain, model/source epoch protection or full post-commit publication
rollback redesign. Those are separate contracts, not guarantees of this slice.

## Actual verification and limits

- Full portable scheduler suite: 133 tests passed, including new rejection,
  ambiguity/duplicate and non-executable-choice cases.
- Scheduler Clippy, all targets with `-D warnings`: passed.
- `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service
  -p pantograph-embedded-runtime --features backend-pytorch --lib --tests`:
  passed without warnings. New production-provider tests assert zero registry
  leases for ambiguous/duplicate alternatives and only the requested candidate's
  first lease when an unrequested alternative appears first. These native tests
  compiled but were not linked or executed locally.
- Formatting, whitespace, critical/accessibility gates, scheduler-only surface
  gate and traceability checker tests passed; 27 accessibility and 28 traceability
  checker tests. Production audit reported zero vulnerabilities.
- Staged and committed-range traceability checked before upload. Semantic review
  remains required; the gate is not a substitute for it.

Logs: `/workspace/pantograph-cache/selected-reservation-*.log`. Standards inspected
at `366c1d90a24bbfb50973f62b155a5f3396c0f107`; existing Rust 1.92.0 toolchain,
regular speed and one Cargo build job. Frontend sources are unchanged and frontend
tests were not rerun for this slice. No GUI, desktop IPC, real model inference or
hardware profiling was executed for this slice. Owner-deferred cloud ORT access
does not block source delivery; no download retry or workaround was attempted.

Continue the separate production observation prerequisite after this milestone.
Parent owns native qualification, coordinated review, integration and publication
metadata; upload only the new source branch under the owner's source-upload approval.

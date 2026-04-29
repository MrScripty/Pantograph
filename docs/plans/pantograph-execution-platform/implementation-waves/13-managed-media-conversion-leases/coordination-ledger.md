# Stage 13 Coordination Ledger

## Current Status

Stage `13` completed Waves `01` through `06`; Wave `07` is partially complete.

## Boundary Snapshot

- Stage `11` completed descriptor attribution, binary-safe ArtifactStore
  payload handling, managed media dependency scaffolding, active-version
  descriptor metadata, GUI/API/binding projections, and stream artifactization.
- Stage `13` owns real converter process execution and per-conversion
  active-version lease-token attribution.
- `pantograph-workflow-service` must remain host-agnostic; conversion process
  execution should live in a host-owned or neutral conversion boundary.
- Managed dependency state and leases currently live under `inference` managed
  redistributable/media dependency modules.
- Ambient active-version snapshots are not sufficient for Stage `13`; actual
  conversion attribution must come from leases acquired immediately before
  converter invocation.

## Required First Actions

1. [x] Apply `08-stage-start-implementation-gate.md`.
2. [x] Inspect current dirty files and confirm Stage `13` write-set safety.
3. [x] Re-read Stage `11` completion notes and Stage `13` plan.
4. [x] Freeze conversion request/result/error and attribution field design.
5. [x] Choose crate/module ownership for the conversion executor.
6. [x] Record non-overlapping worker write sets before launching parallel
   workers.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01-boundary-design` | Complete | Added neutral `pantograph-media-conversion` crate with typed conversion ids, request/result/error contracts, per-conversion dependency attribution, and executor trait. |
| `wave-02-conversion-executor` | Complete | Added process-runner abstraction, managed executable path validation, Tokio-backed standard process runner, bounded stderr summaries, timeout/process failure mapping, and fake-runner tests in `pantograph-media-conversion`. |
| `wave-03-lease-attribution` | Complete | Added attribution holder convention, validation, holder propagation on lease tokens, helper re-exports, and inference tests for multi-dependency acquisition, rollback, release, and invalid holders. |
| `wave-04-media-type-coverage` | Complete for command planning | Added host-neutral command plans for image/audio/video targets, managed dependency requirements, stdin/stdout stream markers, argv vectors, and fail-closed 3D planning. Real conversion execution remains for Wave `06`. |
| `wave-05-conversion-metadata-contracts` | Complete | Added typed conversion status, conversion id, command id, and per-conversion dependency lease attribution fields to artifact descriptors and durable I/O diagnostics metadata. |
| `wave-06-host-conversion-integration` | Complete | Workflow-service has a neutral conversion executor hook; inference exposes typed managed executable resolution; Tauri injects a host adapter that leases active managed dependencies, executes command-plan steps, returns lease attribution, and releases leases on success/failure paths. |
| `wave-07-api-gui-rollout` | Partial | Frontend DTO visibility and I/O Inspector descriptor rendering are implemented for conversion metadata. Managed executable fixture and removal-race coverage are implemented. Conversion failure projection remains pending for failures before a retained artifact descriptor exists. |

## 2026-04-29 Wave 05 Contract Slice

- Current dirty files before this slice remained limited to unrelated deleted
  assets, untracked diagnostics SQLite, and untracked asset files.
- Wave `05` was kept host-owned instead of delegated because it touched shared
  public descriptor and diagnostics contracts that later workers must consume.
- Pass-through constructors in workflow-service, embedded-runtime, and Tauri
  were updated to leave conversion fields empty.
- Verification passed:
  `cargo test -p pantograph-diagnostics-ledger -p pantograph-workflow-service artifact --tests`
  and `cargo check -p pantograph-embedded-runtime -p pantograph`.

## 2026-04-29 Wave 06 Parallel Start

- Current dirty files before this split remained limited to unrelated deleted
  assets, untracked diagnostics SQLite, and untracked asset files.
- Worker A owns only the workflow-service conversion hook and fake-executor
  tests. It must not edit Tauri, inference, or conversion-crate contracts.
- Explorer B is read-only and will identify the host adapter insertion point,
  executable-path resolution risks, and a safe follow-up write set for the
  Tauri/inference adapter.
- Integration sequence: merge workflow-service hook first, review Explorer B
  findings, then launch or implement the host adapter in a separate write set.

## 2026-04-29 Wave 06 Partial Integration

- Worker A completed the workflow-service neutral conversion hook. Format
  override media-type mismatches now call an injected
  `pantograph_media_conversion::MediaConversionExecutor` or fail closed when no
  executor is configured.
- Integration review added command id and lease holder fields to the neutral
  conversion result contract so workflow-service records host-supplied
  conversion attribution instead of deriving lease facts locally.
- Integration review preserved pass-through stream chunk ordering. Converted
  streams are written as one converted output chunk; non-converted streams keep
  their original chunk sequence.
- Explorer B identified that executable paths were only inferable from
  `install_root + expected_files`. A typed resolver was added in `inference` so
  the future Tauri adapter does not depend on a brittle first-file convention.
- Verification passed:
  `cargo test -p pantograph-media-conversion`,
  `cargo test -p pantograph-workflow-service artifact_output_conversion`, and
  `cargo test -p inference --test managed_media_dependencies`.
- Remaining Wave `06` work: implement and inject the Tauri host adapter that
  acquires/release dependency plans, resolves managed executable paths, runs
  command-plan steps, and returns neutral conversion results.

## 2026-04-29 Wave 06 Host Adapter Complete

- Host-owned Tauri slice completed without touching frontend, generated
  bindings, or workflow-service internals after the neutral hook was committed.
- `src-tauri/src/workflow/managed_media_conversion.rs` now maps neutral
  conversion requests to managed dependency lease plans, resolves executable
  paths through `inference`, runs stdin/stdout command-plan steps through the
  neutral `ProcessRunner`, and returns conversion results with dependency id,
  active version, lease id, and lease holder attribution.
- `src-tauri/src/app_setup.rs` injects the adapter into the shared
  `WorkflowService` at startup, using the same `.pantograph` data directory as
  ArtifactStore and managed media dependency activation.
- Focused verification passed:
  `cargo test -p pantograph managed_media_conversion -- --nocapture` and
  `cargo check -p pantograph-embedded-runtime -p pantograph`.
- Existing Tauri dead-code warnings remain outside this slice.

## 2026-04-29 Artifact Metadata Retention Proof

- Host-owned regression coverage added in `pantograph-workflow-service` proving
  conversion metadata remains queryable on ArtifactStore descriptors after a
  delete-on-consume body deletion clears the read handle and marks the artifact
  deleted.
- Verification passed:
  `cargo test -p pantograph-workflow-service descriptor_keeps_conversion_metadata_after_delete_on_consume_removes_body -- --nocapture`.

## 2026-04-29 Wave 07 Conversion Visibility Complete

- Frontend/API visibility pass completed for conversion metadata without Rust,
  generated binding, asset, or `.pantograph` edits. TypeScript DTOs now include
  conversion status, command id, conversion id, and dependency lease
  attribution, and the I/O Inspector artifact descriptor renders those fields
  when present.
- Focused verification passed:
  `node --experimental-strip-types --test src/components/workbench/ioInspectorPresenters.test.ts src/services/workflow/WorkflowService.commands.test.ts src/services/workflow/WorkflowService.projections.test.ts`,
  `npm run typecheck`, and targeted `npx eslint`.

## Proposed Worker Split

Do not launch these workers until Wave `01` freezes shared contracts and the
host records concrete write sets.

| Owner | Scope | Primary Write Set | Forbidden Shared Files | Report |
| ----- | ----- | ----------------- | ---------------------- | ------ |
| Conversion executor worker | Managed process invocation, path validation, timeout, cancellation, stderr truncation, and temp cleanup with fake process-runner tests. | `crates/pantograph-media-conversion/src/**`, `crates/pantograph-media-conversion/Cargo.toml` if a dependency is justified, `reports/wave-02-worker-conversion-executor.md` | `crates/inference/**`, workflow-service files, frontend files, generated bindings, root manifests, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-02-worker-conversion-executor.md` |
| Lease attribution worker | Active-version lease acquisition/release hardening and attribution-ready media dependency plans. | `crates/inference/src/managed_media_dependencies.rs`, `crates/inference/tests/managed_media_dependencies.rs`, `reports/wave-03-worker-lease-attribution.md` | `crates/pantograph-media-conversion/**`, workflow-service files, frontend files, generated bindings, root manifests, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-03-worker-lease-attribution.md` |
| Media command-planning worker | Typed command-plan builders and tests for image/audio/video conversion targets plus fail-closed unsupported 3D planning. | `crates/pantograph-media-conversion/src/**`, crate READMEs, `reports/wave-04-worker-media-command-planning.md` | `crates/inference/**`, workflow-service files, frontend files, generated bindings, root manifests, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-04-worker-media-command-planning.md` |

## 2026-04-29 Wave 04 Worker Split

- Current dirty files before this split remain limited to unrelated deleted
  assets, untracked diagnostics SQLite, and untracked assets.
- Wave `04` is assigned as a single worker because command planning touches the
  same conversion crate contract surface as executor scaffolding.
- The worker must not wire command plans into workflow-service or Tauri; host
  integration remains a later wave.

## 2026-04-29 Wave 02/03 Parallel Worker Split

- Current dirty files before this split remain limited to unrelated deleted
  assets, untracked diagnostics SQLite, and untracked assets.
- Shared contracts in `crates/pantograph-media-conversion/src/lib.rs` are
  frozen for the lease-attribution worker. If that worker needs contract
  changes, it must record the need in its report instead of editing the crate.
- Integration sequence: conversion executor worker first, then lease
  attribution worker, then host-owned docs/status updates and verification.

## 2026-04-29 Wave 01 Start Gate

- Start outcome: `ready_with_recorded_assumptions`.
- Existing dirty files before Stage `13`: deleted image assets, untracked
  `.pantograph/workflow-diagnostics.sqlite`, and untracked asset files. They
  remain outside the Stage `13` write set.
- Standards reviewed: plan, architecture, security, concurrency, Rust, and
  documentation standards.
- Selected boundary: `crates/pantograph-media-conversion`.
- Assumption: later host-owned implementation can adapt `inference` managed
  dependency plans into the neutral crate without making workflow-service
  depend on `inference`.

## Verification Notes

- Wave `01`: `cargo test -p pantograph-media-conversion`,
  `cargo fmt --all -- --check`, and `npm run traceability`.
- Wave `02`: `cargo test -p pantograph-media-conversion`,
  `cargo fmt --all -- --check`, and `npm run traceability`.
- Wave `03`: `cargo test -p inference --test managed_media_dependencies`,
  `cargo fmt --all -- --check`, and `npm run traceability`.
- Wave `04`: `cargo test -p pantograph-media-conversion`,
  `cargo fmt --all -- --check`, and `npm run traceability`.
- Wave `05`: `cargo test -p pantograph-diagnostics-ledger -p pantograph-workflow-service artifact --tests`
  and `cargo check -p pantograph-embedded-runtime -p pantograph`.
- Wave `06` partial: `cargo test -p pantograph-media-conversion`,
  `cargo test -p pantograph-workflow-service artifact_output_conversion`, and
  `cargo test -p inference --test managed_media_dependencies`.
- Wave `07`: Tauri adapter lease cleanup guard completed for dropped/cancelled
  conversion futures. Verification passed:
  `cargo test -p pantograph managed_media_conversion -- --nocapture` and
  `cargo fmt --all -- --check`.
- Wave `07`: Managed executable fixture and dependency removal-race coverage
  passed through `cargo test -p pantograph managed_media_conversion -- --nocapture`.

## Open Decisions

- Exact host adapter ownership for mapping `inference` media dependency plans
  into `pantograph-media-conversion` attribution records.
- Whether lease tokens are stored as raw ids, redacted ids, or stable
  fingerprints after diagnostics projection is wired.
- Whether unsupported 3D conversion starts as fail-closed capability behavior
  or includes a concrete managed tool in the first Stage `13` implementation
  wave.

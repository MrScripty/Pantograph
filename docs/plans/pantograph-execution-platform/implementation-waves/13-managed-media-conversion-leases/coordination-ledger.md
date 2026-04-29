# Stage 13 Coordination Ledger

## Current Status

Stage `13` completed Wave `01` boundary contract freeze.

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
| `wave-04-media-type-coverage` | Planned | Add image/audio/video/supported-3D conversion fixtures and capability validation. |
| `wave-05-api-gui-rollout` | Planned | Surface conversion lifecycle and failures through diagnostics, API, and GUI projections. |

## Proposed Worker Split

Do not launch these workers until Wave `01` freezes shared contracts and the
host records concrete write sets.

| Owner | Scope | Primary Write Set | Forbidden Shared Files | Report |
| ----- | ----- | ----------------- | ---------------------- | ------ |
| Conversion executor worker | Managed process invocation, path validation, timeout, cancellation, stderr truncation, and temp cleanup with fake process-runner tests. | `crates/pantograph-media-conversion/src/**`, `crates/pantograph-media-conversion/Cargo.toml` if a dependency is justified, `reports/wave-02-worker-conversion-executor.md` | `crates/inference/**`, workflow-service files, frontend files, generated bindings, root manifests, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-02-worker-conversion-executor.md` |
| Lease attribution worker | Active-version lease acquisition/release hardening and attribution-ready media dependency plans. | `crates/inference/src/managed_media_dependencies.rs`, `crates/inference/tests/managed_media_dependencies.rs`, `reports/wave-03-worker-lease-attribution.md` | `crates/pantograph-media-conversion/**`, workflow-service files, frontend files, generated bindings, root manifests, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-03-worker-lease-attribution.md` |
| Media fixture worker | Fixture/golden tests for image/audio/video/3D conversion and capability coverage. | To be assigned after Wave `01`. | Production contracts, frontend files, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-04-worker-media-fixtures.md` |

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

## Open Decisions

- Exact host adapter ownership for mapping `inference` media dependency plans
  into `pantograph-media-conversion` attribution records.
- Whether lease tokens are stored as raw ids, redacted ids, or stable
  fingerprints after diagnostics projection is wired.
- Whether unsupported 3D conversion starts as fail-closed capability behavior
  or includes a concrete managed tool in the first Stage `13` implementation
  wave.

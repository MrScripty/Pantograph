# 13: Managed Media Conversion Leases

## Purpose

Implement real managed media conversion/transcoding for ArtifactStore-backed
outputs and record per-conversion active-version lease attribution.

Stage `11` made artifact bodies binary-safe and attached selected format plus
active managed dependency metadata to descriptors. It intentionally did not
invoke external conversion tools. This stage closes that remaining gap by
adding a host-owned conversion boundary that uses only Pantograph-managed media
dependencies.

## Objective

Pantograph converts image, audio, video, and 3D artifacts through managed
dependencies without exposing system PATH tools, raw filesystem paths, or
storage-tier details to clients. Every real conversion acquires dependency
leases immediately before tool invocation, records the exact leased versions in
the resulting artifact metadata and diagnostics, and releases leases on success,
failure, or cancellation.

## Scope

In scope:

- Host-owned conversion executor boundary for `ffmpeg`, `ocioconvert`, and
  `oiiotool`.
- Per-conversion active-version leases for all managed tools and OCIO artifacts
  used by a conversion.
- ArtifactStore temporary-input and output-body handling that keeps physical
  paths private to the backend.
- Typed conversion requests, results, errors, cancellation, timeout, and cleanup
  behavior.
- Descriptor metadata updates for actual converter id, tool version, library or
  profile version, output format, codec, quality/compression, bitrate, color
  profile/transform, bit depth, and 3D format.
- Diagnostics events and projections that can distinguish pass-through
  descriptor attribution from real conversion.
- Tests proving managed dependency activation/removal races fail closed or wait
  on active leases without corrupting artifacts.

Out of scope:

- Unmanaged host PATH discovery or system package probing.
- Raising workflow JSON value limits for media bodies.
- Iroh distributed artifact transfer.
- Per-workflow/per-artifact retention policy.
- GPU-accelerated encoder selection beyond backend capability metadata.

## Current Code Compatibility Findings

- `crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
  performs descriptor attribution, base64/data-url decoding, ArtifactStore
  writes, selected format metadata resolution, and pass-through media-type
  validation. When a selected output format requires real transcoding, current
  code fails closed because transcoding is not implemented.
- `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
  records selected converter/library version strings, but it does not yet have
  a per-conversion dependency attribution list tied to actual lease tokens.
- `src-tauri/src/workflow/headless_workflow_commands.rs` synchronizes ambient
  active managed dependency versions into workflow-service capabilities. That
  is useful for descriptor metadata, but it is not a substitute for acquiring
  leases around a real conversion process.
- `crates/inference/src/managed_media_dependencies.rs` and
  `crates/inference/src/managed_redistributables/operations.rs` own managed
  media dependency status, activation, and lease planning. They are the correct
  source for conversion dependency leases, but the plan must harden
  multi-dependency attribution and stale lease behavior before long-running
  conversions depend on it.
- `crates/inference/src/managed_redistributables/catalog.rs` currently models
  `ffmpeg`, `ocioconvert`, `oiiotool`, and OpenColorIO catalog/status data.
  Current readiness is expected-file oriented; executable behavior, codec
  support, checksum/signature enforcement, and OCIO ABI validation still need
  concrete verification before capabilities can claim real conversion support.
- The clean architectural path is a new host-owned conversion boundary such as
  `crates/pantograph-media-conversion` or a neutral conversion module that
  consumes `inference` managed dependency state through explicit traits.
  Process spawning and managed dependency leases must not be placed directly in
  `pantograph-workflow-service`.

## Implementation Progress

### 2026-04-29 Stage-Start And Boundary Contract Slice

Wave status: `complete`.

- Start outcome: `ready_with_recorded_assumptions`.
- Dirty-file review found only unrelated deleted asset files, untracked
  diagnostics SQLite, and untracked asset files. They do not overlap Stage
  `13` and remain untouched.
- Standards reviewed before source edits: plan, architecture, security,
  concurrency, Rust, and README/documentation standards.
- Boundary decision: add a neutral `pantograph-media-conversion` crate for
  conversion request/result/error contracts, typed ids, target format metadata,
  per-conversion dependency attribution, and the executor trait. This keeps
  process execution and managed dependency leases outside
  `pantograph-workflow-service`.
- Current implementation freezes contract types only. Real process invocation,
  dependency lease acquisition/release, ArtifactStore temporary-file handling,
  diagnostics propagation, and GUI status projection remain assigned to later
  Stage `13` waves.
- Verification passed:
  `cargo test -p pantograph-media-conversion`,
  `cargo fmt --all -- --check`, and `npm run traceability`.

### 2026-04-29 Executor And Lease Attribution Scaffolds

Wave status: `complete`.

- Wave `02` added the `pantograph-media-conversion` process-runner abstraction,
  validated managed executable paths, Tokio-backed standard process execution,
  bounded stderr summaries, timeout mapping, and a
  `ManagedProcessConversionExecutor` bridge covered by fake-runner tests.
- Wave `03` hardened `inference` media conversion dependency plans with the
  holder convention
  `workflow_run:{workflow_run_id}/node:{node_id}/port:{port_id}/conversion:{conversion_id}`,
  holder validation before lease acquisition, holder propagation on lease
  tokens, public helper re-exports, and tests for multi-dependency acquisition,
  rollback, release, and malformed holders.
- Real converter-specific argument planning, ArtifactStore private temporary
  file handoff, workflow-service integration, descriptor/diagnostic propagation,
  and GUI status projection remain assigned to later Stage `13` waves.
- Verification passed:
  `cargo test -p pantograph-media-conversion`,
  `cargo test -p inference --test managed_media_dependencies`,
  `cargo fmt --all -- --check`, and `npm run traceability`.

## Milestones

### Milestone 1: Conversion Boundary Design

- [x] Define the host-owned conversion executor interface and keep
  `pantograph-workflow-service` host-agnostic.
- [x] Prefer a new neutral crate/module, for example
  `crates/pantograph-media-conversion`, unless source inspection during this
  milestone proves an existing host/runtime boundary is cleaner.
- [x] Decide the concrete owner for process spawning and managed dependency lease
  acquisition. The default target is a host/runtime boundary that can access
  `inference` managed media state and ArtifactStore internals through explicit
  service methods.
- [x] Define pass-through versus real-conversion decision rules.
- [x] Define conversion attribution fields before implementation begins,
  including
  conversion id, converter id, dependency ids, activated versions, lease token
  fingerprints or ids safe for diagnostics, and conversion status.
- [x] Record allowed write sets before launching workers.

Verification:

- Source audit confirms conversion code does not add an `inference` dependency
  to `pantograph-workflow-service`.
- Contract tests prove conversion requests/results are typed and bounded.

### Milestone 2: Managed Tool Invocation

- [x] Implement safe process invocation scaffolding for host-resolved managed
  executable paths.
- [x] Reject empty, relative, control-character, and shell-metacharacter
  executable paths before process launch.
- [x] Add timeout, cancellation, stderr truncation, and process failure error
  mapping to the conversion contracts and fake-runner coverage.
- [ ] Add converter-specific `ffmpeg`, `ocioconvert`, and `oiiotool` argument
  planning and private temporary-file cleanup for tools that cannot operate as
  stdin/stdout filters.
- [ ] Reject inactive, missing, incompatible, or removed dependency versions at
  the host conversion boundary before process launch.

Verification:

- Unit tests cover path rejection, missing executable, non-zero exit, timeout,
  cancellation cleanup, and bounded error output.
- No conversion path shells through user-supplied command strings.
- Tests use a fake process runner for deterministic unit coverage before any
  fixture-based real binary tests are added.

### Milestone 3: Lease Attribution And Artifact Metadata

- [x] Add attribution-ready holder convention and validation for
  active-version lease plans.
- [x] Preserve dependency id, active version, lease id, holder, install root,
  and expected files for acquired media conversion dependency plans.
- [x] Prove multi-dependency rollback and release behavior before real
  converter invocation is wired.
- [ ] Acquire active-version leases immediately before invoking a converter.
- [ ] Record leased tool/library/profile versions, lease ids, and converter
  command identity in ArtifactStore descriptor metadata and durable I/O
  diagnostics.
- [ ] Release leases on success, failure, cancellation, and dropped futures in
  the host conversion executor.
- [ ] Preserve queryable metadata after retention deletes the physical body.

Verification:

- Race tests cover activate/remove while conversion is running.
- Tests cover multi-dependency conversion plans, including image conversion
  requiring `oiiotool`, `ocioconvert`, and OpenColorIO readiness together.
- Descriptor and diagnostics tests prove lease attribution survives restart and
  body deletion.

### Milestone 4: Media-Type Implementations

- [ ] Add image conversion using managed OpenImageIO/OpenColorIO tooling.
- [ ] Add audio/video conversion using managed ffmpeg.
- [ ] Add 3D conversion for GLB/glTF/OBJ where managed tooling exists; otherwise
  fail closed with typed unsupported-conversion errors.
- [ ] Keep streamed previews and in-progress streams readable through ArtifactStore
  handles while final converted artifacts are produced.

Verification:

- Golden or fixture tests prove output descriptors match selected formats
  without embedding media bodies in JSON.
- Capability tests prove unsupported converter/format pairs are not advertised
  as executable.

### Milestone 5: API, GUI, And Rollout

- [ ] Surface conversion status and failures through existing run diagnostics and
  I/O Inspector artifact lifecycle fields.
- [ ] Keep the workbench Settings page as the canonical owner of conversion
  defaults and managed dependency activation controls.
- [ ] Add a stage-end standards/refactor review before marking this stage complete.

Verification:

- Focused backend, Tauri, binding, and frontend checks pass for descriptor
  reads, conversion errors, and Settings-driven defaults.
- `npm run traceability` passes after docs and reports are updated.

## Concurrent Worker Plan

Use workers only after Milestone 1 freezes shared conversion contracts.

| Owner | Scope | Primary Write Set | Forbidden Shared Files | Report |
| ----- | ----- | ----------------- | ---------------------- | ------ |
| Conversion executor worker | Implement managed process invocation, path validation, timeout, cancellation, and cleanup helpers. | Host/runtime conversion executor module selected by Milestone 1, focused executor tests. | ArtifactStore contract DTOs, frontend files, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `implementation-waves/13-managed-media-conversion-leases/reports/wave-02-worker-conversion-executor.md` |
| Lease attribution worker | Implement active-version lease acquisition/release around conversion and descriptor/diagnostics metadata propagation. | Managed dependency lease module, conversion metadata adapters, focused race tests. | Frontend files, process-spawn helpers unless assigned, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `implementation-waves/13-managed-media-conversion-leases/reports/wave-03-worker-lease-attribution.md` |
| Media fixture worker | Add fixture-based conversion tests and capability coverage for image/audio/video/3D. | Test fixtures under approved test-data paths, focused tests, worker report. | Production contracts, frontend files, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `implementation-waves/13-managed-media-conversion-leases/reports/wave-04-worker-media-fixtures.md` |

Integration sequence: executor first, lease attribution second, media fixtures
third, then host-owned API/frontend status integration and stage-end gate.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Conversion code depends on system PATH tools. | High | Resolve executable paths only from activated managed dependency state and test arbitrary path rejection. |
| Workflow-service becomes coupled to host dependency management. | High | Keep workflow-service host-agnostic; inject conversion capability through a host boundary. |
| Activation/removal races invalidate running conversions. | High | Require active-version leases, per-dependency transition locks, and race tests. |
| Temporary files leak raw artifact paths or bodies. | High | Keep temp paths private to backend-owned storage and clean up on every exit path. |
| Diagnostics receive large media bodies or unbounded stderr. | High | Store only descriptors, bounded metadata, and truncated error summaries. |
| Capability DTOs advertise unsupported conversions. | Medium | Derive executable capabilities from active dependency readiness and tested converter support. |

## Re-Plan Triggers

- A required converter cannot be invoked without unmanaged host PATH discovery.
- OpenColorIO/OpenImageIO integration requires a native ABI boundary instead of
  managed `ocioconvert`/`oiiotool` process invocation.
- Conversion needs long-running worker processes instead of bounded tool
  invocations.
- ArtifactStore cannot provide private temporary inputs/outputs without a
  storage API refactor.
- A binding or GUI consumer requires raw media bodies in JSON.

## Completion Criteria

- Real image, audio, video, and supported 3D conversions use managed
  dependencies only.
- Every conversion records per-conversion active-version lease attribution in
  artifact descriptors and diagnostics.
- Conversion failures are typed, bounded, and visible in diagnostics without
  leaking raw paths or media bodies.
- Activation/removal, cancellation, timeout, and retention races are covered by
  tests.
- Workbench Settings remains the canonical owner for persistent format defaults
  and managed dependency activation.
- Stage-end standards/refactor gate is recorded before the next stage begins.

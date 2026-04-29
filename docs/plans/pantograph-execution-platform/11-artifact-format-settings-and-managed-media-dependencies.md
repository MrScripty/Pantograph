# 11: Artifact Format Settings And Managed Media Dependencies

## Purpose

Extend the execution platform with standards-compliant artifact payload
handling, persistent artifact format settings, and backend-managed media
dependencies for image, audio, video, and 3D outputs.

This stage turns the recent artifact and settings decisions into the active
execution-platform implementation plan. It is intentionally separate from the
completed Stage `03` and Stage `04` closeouts because it changes the execution
platform beyond the original model/license ledger scope.

## Objective

Pantograph must move large media and binary workflow payloads through a
backend-owned ArtifactStore instead of inline JSON. The GUI and external
clients receive descriptors and binary-safe retrieval or streaming handles. The
workbench Settings page owns persistent artifact format defaults and managed
dependency controls. Runtime execution, diagnostics, bindings, and GUI surfaces
consume backend-owned typed contracts.

## Scope

In scope:

- ArtifactStore descriptors, retrieval, streaming, retention state, and consume
  acknowledgement contracts for image, audio, video, 3D, large table, and
  generic binary payloads.
- Persistent artifact format defaults for image, audio, video, and 3D outputs.
- Per-output-node format overrides validated against backend capabilities and
  captured in run snapshots, artifact descriptors, and diagnostics metadata.
- Workbench Settings page as the canonical persistent settings owner.
- Managed redistributables boundary for runtime sidecars, tool binaries, and
  native library artifacts.
- Library-managed OpenColorIO dependency activation and isolated OCIO
  FFI/native-library loading.
- Library-managed `ffmpeg`, `ocioconvert`, and `oiiotool` tool binaries.
- Source audit and migration of existing base64/data-url media paths.
- Binding/API projection updates for artifact descriptors, binary-safe body
  retrieval/streaming, Settings, capabilities, and managed dependency status.

Out of scope:

- Iroh distributed artifact transfer.
- Per-workflow, per-run, or per-artifact retention policy beyond future
  extensibility.
- User-facing visual redesign beyond moving persistent settings to the
  workbench Settings page.
- Supporting legacy inline media JSON behavior after the breaking cutover.

## Current Code Compatibility Findings

- `crates/inference/src/managed_runtime` currently owns sidecar runtime
  redistributables for llama.cpp/Ollama. It is the implementation pattern, but
  not a sufficiently broad product model for OCIO, ffmpeg, `ocioconvert`, or
  `oiiotool`.
- Current media paths still include base64/data-url payloads in workflow nodes,
  Python workers, embedded-runtime bridges, and Svelte node components. The
  ArtifactStore cutover must audit and migrate these paths explicitly.
- Current workflow value validation correctly caps JSON payload size. The fix
  for images, audio, video, and 3D output is not raising that cap; it is moving
  payload bodies behind ArtifactStore descriptors and binary-safe body APIs.
- Existing Settings-related surfaces are split across side panel, server
  status, and runtime-manager UI. The workbench Settings page must become the
  canonical owner for persistent settings, with legacy surfaces relocated,
  embedded, or retired.

## Implementation Progress

### 2026-04-29 Stage-Start Preflight

Start outcome: `ready_with_recorded_assumptions`.

- Selected stage: Stage `11`, artifact format settings and managed media
  dependencies.
- Plan, Stage `07` standards review, Stage `08` start gate, and Stage `10`
  concurrent phased implementation rules were read before source edits.
- Standards reviewed for this stage: coding, architecture, dependency,
  security, frontend, language bindings, Rust standards, and Rust unsafe
  standards. Commit standards apply to every logical slice.
- Dirty-file review: unrelated deleted `.pantograph` workflow/asset files,
  untracked diagnostics SQLite, and untracked asset files are present before
  this stage. They do not overlap Stage `11` and must not be staged,
  reformatted, or reverted.
- Required first wave: `preflight-contract-audit`. Source implementation must
  start with backend-owned contract/DTO freeze before ArtifactStore,
  redistributable, execution, binding, or frontend workers are launched.
- Initial source audit confirms existing inline media paths in workflow nodes,
  Python bridges, embedded-runtime tests, frontend input/output nodes, C#
  smoke helpers, and legacy LLM/VLM commands. The migration rule remains:
  convert media bodies to ArtifactStore descriptors and binary-safe body
  access rather than raising `max_value_bytes`.
- Concurrency decision: use parallel agents only after contracts are frozen and
  wave specs assign non-overlapping write sets. The host owns shared contracts,
  coordination ledger updates, and integration commits.
- Expected first-slice verification:
  `cargo test -p pantograph-workflow-service --test artifact_contract`,
  `cargo test -p pantograph-workflow-service --test contract`,
  `cargo fmt --all -- --check`, and targeted source-audit documentation
  review. Broader verification is added as later waves touch runtime,
  diagnostics, bindings, frontend, or dependency manifests.

### 2026-04-29 Wave 01 Contract Freeze

Wave status: `complete`.

- Added backend-owned ArtifactStore DTOs in
  `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
  instead of expanding the already-large workflow `contracts.rs`.
- Added backend-owned media capability and managed redistributable DTOs in
  `crates/pantograph-workflow-service/src/workflow/media_capability_contracts.rs`.
- Public workflow-service exports now include artifact descriptors, lifecycle
  states, handle-based read/stream/consume contracts, global artifact policy,
  format defaults, media capability options, and managed redistributable
  product categories.
- Contract snapshots prove descriptor responses carry metadata and handles,
  not data URLs or inline media payload bodies.
- Validation implementation for settings bounds, handles, ids, and managed
  dependency catalogs remains assigned to later backend waves; this wave freezes
  the public shape those implementations must satisfy.
- Verification passed:
  `cargo test -p pantograph-workflow-service --test artifact_contract`,
  `cargo test -p pantograph-workflow-service --test contract`, and
  `cargo fmt --all -- --check`.

### 2026-04-29 Wave 02 ArtifactStore Core Slice

Wave status: `in_progress`.

- Added a backend ArtifactStore owner in
  `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`.
- The store persists artifact bodies under private backend-owned paths, writes
  descriptors with handles rather than raw filesystem locations, reconciles
  missing bodies on restart as metadata-only records, enforces artifact id and
  single-artifact size validation, supports byte-range body reads through an
  internal binary body return type, tracks consume acknowledgement, and deletes
  physical bodies while preserving metadata for retention cleanup.
- Added `crates/pantograph-workflow-service/src/workflow/artifact_api.rs` so
  `WorkflowService` owns descriptor lookup, binary body reads, consume
  acknowledgement, policy update, cleanup, and stats through a configured
  ArtifactStore.
- Added focused ArtifactStore tests for path opacity, restart reconciliation,
  invalid id rejection, size limits, consume deletion, TTL cleanup, and service
  facade access.
- Residual Wave `02` work: memory-cache accounting/enforcement beyond
  persisted body stats, stream body persistence, finalize lifecycle transitions,
  integration with execution output conversion, and diagnostics metadata
  linking.
- Verification passed:
  `cargo test -p pantograph-workflow-service --test artifact_store`,
  `cargo test -p pantograph-workflow-service --test artifact_contract`,
  `cargo test -p pantograph-workflow-service --test contract`,
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`,
  and `cargo fmt --all -- --check`.

## Architecture Decisions

### ArtifactStore Boundary

ArtifactStore owns physical payload bodies, memory/disk cache state, streaming
chunk ordering, finalization, consume acknowledgement state, size accounting,
and physical cleanup.

Diagnostic events, run snapshots, workflow outputs, and binding DTOs carry
descriptors only:

- artifact id
- payload kind
- media type or MIME/container id
- byte length
- content hash
- lifecycle state
- retention state
- stream/download/read handle
- run/workflow/node attribution
- actual format/codec/conversion settings

Clients must not receive raw filesystem paths, storage-tier details, or inline
base64 media bodies in normal workflow/API JSON.

### Canonical Settings Owner

The workbench Settings page is the canonical owner for persistent GUI and
framework settings. Artifact format defaults, retention/cache policy, managed
dependency installation/selection/activation, runtime-manager settings, and
server connection settings must be relocated into or exposed through that page.

Feature-local helper settings may remain near their feature only when their
placement is directly tied to usability, such as search filters or table
filters, and they do not own Pantograph-wide persistent configuration.

### Format Defaults

Backend-owned defaults:

- image: OpenImageIO-supported formats, OpenColorIO color management, default
  JPEG at 75 percent quality, sRGB color profile.
- audio: ffmpeg-backed Ogg Opus at 96 kbps by default, with Vorbis, WAV, MP3,
  AIFF, and FLAC support where capabilities allow.
- video: ffmpeg-backed SVT-AV1 CRF 32 8-bit by default.
- 3D: GLB by default, with glTF and OBJ support.

Format DTOs must use typed ids, explicit units, and backend-validated bounds:
quality range, bitrate unit, CRF range, bit-depth enum, MIME/container id,
codec id, color profile id/source, 3D format id, and active converter/library
version fields. Backend-authored display labels are allowed; frontend-created
option lists are not.

### Managed Redistributables Boundary

The existing managed-runtime system is the reference pattern, not the final
name or contract for every managed artifact. Implementation must generalize or
split the boundary before adding media dependencies if runtime-only naming
would leak into tool or library contracts.

The boundary must model at least:

- runtime sidecars: llama.cpp, Ollama, future runtimes;
- tool binaries: `ffmpeg`, `ocioconvert`, `oiiotool`;
- native library/artifact dependencies: OpenColorIO.

Shared infrastructure should cover catalog lookup, download/resume, checksum
or signature validation, license/redistribution metadata, archive extraction
validation, expected-file validation, install/remove transitions,
selected/default version state, activation, capability projection, and restart
recovery.

Required platform policy follows the current managed-runtime matrix unless a
new ADR changes it: Linux x86_64 and Windows x86_64 are required; macOS x86_64
and arm64 are best effort with explicit degraded-state reporting.

### OCIO Native Boundary

OpenColorIO is consumed as a Library-managed dependency, not a Rust-native
crate and not an unmanaged system install.

OCIO binding/loading is a native-library/FFI boundary:

- unsafe code, if required, is isolated in a thin adapter module or crate;
- domain logic never contains FFI mechanics;
- a safe wrapper documents ownership, lifetime, threading, shutdown, and error
  behavior;
- activated artifact version/ABI compatibility is validated before use;
- missing, incompatible, or inactive OCIO fails closed through typed capability
  and validation errors.

### Concurrency And Lifecycle

Install, activation, removal, conversion, ArtifactStore reads, retention cleanup,
and streaming can overlap. The implementation must use per-dependency
transition locks and active-version leases so running conversions or streams
are not invalidated by cleanup or activation changes.

Artifact retention cleanup must preserve durable metadata and audit events even
when physical payload bodies are deleted.

## Tasks

### Milestone 1: Contract And Source Audit Freeze

- [ ] Audit current media paths for `base64`, `image_base64`, `audio_base64`,
  data URLs, oversized media JSON, and workflow outputs carrying binary bodies.
- [ ] Record each producer/consumer migration decision: ArtifactStore
  descriptor, binary-safe stream/read path, or removal during the breaking
  cutover.
- [ ] Define ArtifactStore descriptor, lifecycle, retention, stream, consume,
  read/download, and error contracts.
- [ ] Define artifact format settings and capability DTOs with typed ids,
  units, bounds, and active converter/library version fields.
- [ ] Define managed redistributable product-category DTOs for runtime sidecar,
  tool binary, and native library/artifact dependency.

### Milestone 2: Backend Storage And Policy

- [x] Implement ArtifactStore ownership for physical payload bodies, memory/disk
  cache accounting, spill/finalize lifecycle, restart recovery, and cleanup.
- [x] Persist global artifact policy values: TTL, maximum disk bytes, maximum
  memory bytes, maximum single artifact bytes, spill threshold, delete-on-
  consume behavior, and cleanup status.
- [ ] Store artifact metadata and references in durable diagnostics/run
  projections without placing payload bodies in diagnostic event JSON.
- [ ] Preserve queryable audit metadata after physical payload deletion.

### Milestone 3: Managed Redistributables

- [x] Generalize or split the managed-runtime boundary so tool binaries and
  native library artifacts do not use misleading runtime-only contracts.
- [x] Add managed redistributable catalog metadata for source owner,
  license/redistribution, checksum or signature, platform, archive kind,
  expected files, compatibility, and release version.
- [x] Add OpenColorIO as a managed native library/artifact dependency with
  install/select/activate/capability state.
- [x] Add OCIO safe wrapper and native loading/ABI validation boundary.
- [x] Add `ffmpeg`, `ocioconvert`, and `oiiotool` as managed tool binaries with
  install/select/activate/capability state.
- [x] Ensure conversion jobs resolve only activated managed tool/library
  versions and hold active-version leases while running.

### Milestone 4: Execution And Diagnostics Integration

- [ ] Convert image, audio, video, 3D, large table, generic file, and oversized
  structured outputs to descriptors before workflow-output value validation.
  - [x] Convert workflow-service `image` and `audio` output bindings to
    ArtifactStore descriptors before `max_value_bytes` validation.
  - [ ] Convert video, 3D, large table, generic file, Python bridge streaming,
    and remaining oversized structured outputs.
- [ ] Represent streaming assets as artifact lifecycle transitions. Diffusion
  preview passes should be child or revision artifacts; audio/video streams
  should publish chunk metadata and stream handles without placing chunk bodies
  in diagnostic event JSON.
- [ ] Capture actual output format, codec, quality/compression, bitrate, color
  transform/profile, 3D format, and active converter/library versions in
  artifact descriptors, run snapshots, and diagnostic metadata.
- [ ] Reject invalid format/codec/quality/bitrate/color/3D settings at
  submission or execution boundaries with typed errors.

### Milestone 5: API, Binding, And GUI Projections

- [ ] Expose ArtifactStore descriptor lookup, binary read/download, stream
  subscription/read, consume acknowledgement, and policy commands through the
  GUI/API boundary.
- [ ] Expose Settings APIs for persistent artifact format defaults and
  conversion capabilities.
- [ ] Expose managed redistributable status/actions for OCIO, ffmpeg,
  `ocioconvert`, and `oiiotool` with degraded/missing/incompatible states.
- [ ] Update native Rust and supported host bindings with DTO parity tests for
  artifact descriptors, settings, capabilities, managed dependency status, and
  binary-safe payload access semantics.
- [ ] Make the workbench Settings page the canonical persistent settings
  surface. Relocate, embed, or retire old side-panel/server/runtime settings
  surfaces so they do not keep separate global settings ownership.
- [ ] Add output-node format selectors that default from backend Settings and
  preserve explicit per-node overrides in run snapshots.

## Verification

Required checks before completion:

- Source-audit report or tests covering current `base64`, `image_base64`,
  `audio_base64`, data-url, and oversized media JSON paths.
- ArtifactStore restart/recovery tests for memory-to-disk spill, descriptor
  reconciliation, missing-body recovery, and storage-tier opacity.
- Binary-safe retrieval/streaming tests proving image, audio, video, 3D, large
  table, and generic binary bodies are not transported as inline JSON.
- Streaming lifecycle tests for declared, streaming, finalizing, retained,
  failed, expired, and deleted artifact states.
- Retention cleanup and consume-acknowledgement tests proving physical payload
  deletion preserves queryable metadata and audit events.
- Format policy tests proving default JPEG quality 75 with sRGB, default Ogg
  Opus 96 kbps, default SVT-AV1 CRF 32 8-bit, default GLB, and rejected invalid
  format/codec/quality/bitrate/color settings.
- Managed redistributable catalog tests covering checksum/signature validation,
  license/redistribution metadata, source ownership, archive validation,
  expected files, platform support, and rejected arbitrary download URLs.
- Library-managed OCIO tests covering install/remove/select/activate,
  artifact readiness, incompatible version handling, restart recovery, and
  fail-closed color-management validation.
- OCIO native-library/FFI tests covering safe wrapper conversion, missing
  library, incompatible ABI/version, shutdown behavior, and absence of unsafe
  code outside the adapter boundary.
- Library-managed binary tests covering ffmpeg, `ocioconvert`, and `oiiotool`
  install/remove/select/activate, executable readiness, incompatible version
  handling, restart recovery, and fail-closed conversion validation.
- Concurrency tests covering dependency install/activation locks,
  active-version leases for conversions, retention cleanup races with active
  reads/streams, and cancellation-safe cleanup.
- Settings page tests proving persistent settings are reachable through the
  workbench Settings page and retired/embedded legacy settings surfaces do not
  keep separate global state ownership.
- Cross-layer acceptance proving backend capability DTOs drive frontend option
  lists and output-node selectors without frontend hard-coded media formats.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Artifact implementation raises JSON value limits instead of moving media to ArtifactStore. | High | Require tests proving media/binary outputs become descriptors and binary-safe retrieval/stream APIs carry bodies. |
| Existing base64 media paths survive the cutover. | High | Require source audit and migration checklist before completion. |
| Managed dependency implementation keeps runtime-only names for tools/libraries. | High | Generalize or split the redistributables boundary before adding OCIO/ffmpeg/OIIO surfaces. |
| OCIO FFI code spreads into domain logic. | High | Require a thin unsafe/native adapter boundary, safe wrapper, ABI checks, and targeted verification before image encoding uses OCIO. |
| Managed dependency catalogs omit supply-chain metadata. | High | Require checksum/signature, license/redistribution, source-owner, expected-files, and platform metadata in catalog tests. |
| Activation/removal races with conversions or artifact streams. | High | Require per-dependency locks, active-version leases, cancellation, and cleanup tests. |
| Settings work leaves multiple global persistent settings owners. | High | Require the workbench Settings page to own persistent settings and prove legacy surfaces are embedded, relocated, or retired. |
| Format defaults drift between Settings, output nodes, and encoders. | High | Serve options from backend capability DTOs and validate overrides at submission/execution boundaries. |

## Re-Plan Triggers

- Artifact payload bodies require client-visible raw paths, storage-tier details,
  or inline JSON transport.
- OCIO color management cannot be represented through managed dependency
  activation and would require unmanaged host-library discovery.
- Conversion tooling requires unmanaged host PATH discovery instead of managed
  binary activation.
- Streaming artifacts cannot be modeled without high-volume diagnostic events.
- Managed redistributables cannot be generalized or split without a broader
  crate/module ownership refactor.
- Existing Settings surfaces cannot be relocated, embedded, or retired without
  keeping separate persistent settings ownership.
- Required platform support for media dependencies differs from the current
  managed-runtime support matrix.

## Completion Criteria

- Artifact bodies are backend-owned and never transported as inline JSON for
  image, audio, video, 3D, large table, or generic binary outputs.
- Artifact descriptors, lifecycle, retention, streaming, consume, and binary
  read/download contracts are implemented and tested.
- Artifact format defaults are backend-owned persistent settings, validated
  against conversion capabilities, and captured when used.
- Workbench Settings is the canonical persistent settings page.
- OCIO is a managed native library/artifact dependency with isolated FFI/native
  loading and fail-closed validation.
- ffmpeg, `ocioconvert`, and `oiiotool` are managed tool binaries with
  activated capability state.
- Managed redistributable catalogs include supply-chain and platform metadata.
- Existing base64/data-url media producers are migrated or removed.
- API/binding/frontend projections use descriptors and capability DTOs rather
  than raw storage rows, host PATH probing, or frontend hard-coded options.

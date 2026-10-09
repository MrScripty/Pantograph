# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

### Added
- Tooling hooks and quality-gate scripts for linting, type checking, and tests.
- Tauri path-boundary regression tests for workflow loading, sandbox validation, and agent file tools.
- Canonical composed-node contracts, `tool-loop` composed authoring metadata,
  runtime composed-parent lineage projection, and migration-aware workflow graph
  canonicalization records.

### Changed
- Rust source API: `LlamaServer::start_sidecar_inference` and
  `matches_inference_runtime` now borrow the existing `LlamaCppRuntimeSettings`
  for effective device/context/thread/batch values. Model/mmproj paths, process
  spawner and port override retain their existing roles and ownership.
- Rust source API: PyTorch `generate_with_top_k` and `generate_stream_with_top_k`
  now accept `PyTorchTextGenerationRequest` with the same seven named inputs.
  Legacy `generate`/`generate_stream` signatures and the worker wire contract
  remain unchanged.
- Rust source API: `resolve_managed_binary_command` and
  `resolve_task_registry_entry_from_evidence` now return boxed concrete errors.
  Error variants, diagnostic JSON and Display text remain unchanged; callers
  matching an owned error must dereference the box.
- Rust source API: `ManagedRuntimeCommandResolutionError::MissingRuntimeVariant.diagnostic`
  and `ImageGenerationPlanningOutcome::Planned.plan` now hold boxed payloads.
  Direct constructors must box their values and consuming plan callers must unbox;
  tagged JSON, diagnostic fields, and error Display text are unchanged.
- Rust source API: `MediaConversionResult::try_new` now accepts one
  `MediaConversionResultInput` with the same eight named fields instead of eight
  positional arguments. Result fields and validation behavior are unchanged.
- Rust source API: `SchedulerTaskExecutionIntent::Runtime.task_intent` is now boxed.
  Construct runtime variants with `SchedulerTaskExecutionIntent::runtime(intent)`;
  direct struct-variant callers must pass `Box::new(intent)`. Serialized JSON and
  borrowed `runtime_task_intent()` access remain unchanged.
- Root project `README.md` reorganized around install, usage, development, and contribution workflows.
- Documentation consolidated around current guides, accepted decisions, audits,
  and active plans; superseded narration remains available in Git history.
- Accessibility interaction semantics improved by replacing suppressed non-semantic handlers with button-based interactions.

### Fixed
- Launcher contract behavior aligned with CLI standards and expected error handling paths.

### Security
- Canonical path validation enforced at file-boundary entry points to block traversal and symlink escape paths.

### Runtime-host borrowed accessor compatibility

The seven validated runtime-host contract wrappers now implement standard `AsRef<Raw>` in place of inherent `as_ref` methods. Borrowed values and lifetimes, wrapper types, validation and `into_inner` are unchanged. Ordinary method syntax, qualified calls and function pointers continue to resolve through the standard prelude. Rust callers using `no_implicit_prelude` must explicitly import `std::convert::AsRef`. The workspace crate remains unpublished (`publish = false`).

### Diagnostic event inference payload construction

`DiagnosticEventPayload::InferenceExecutionDiagnosticObserved` now stores `Box<InferenceExecutionDiagnosticObservedPayload>`. Rust callers constructing this public variant must wrap the existing raw payload in `Box::new`; owned pattern bindings now contain a box. Tagged serialized JSON, raw payload fields and validation remain unchanged. This adds one allocation for this event variant; no measured performance improvement is claimed. The crate remains `publish = false`.

### Diagnostic event artifact payload construction

`DiagnosticEventPayload::IoArtifactObserved` now stores `Box<IoArtifactObservedPayload>`. Rust callers constructing the public variant use `Box::new`, and owned pattern bindings contain a box. Raw DTO fields, validation and tagged JSON stay unchanged; this adds one allocation for this variant without a measured performance claim. The crate remains `publish = false`.

### Workflow ready inference projection construction

`WorkflowSchedulerInferenceTaskProjection::Ready` now holds `Box<WorkflowSchedulerReadyInferenceTaskProjection>`. Rust constructors use `Box::new`, and owned pattern bindings contain a box; the raw record, equality, borrowed readers and downstream scheduler intent serialization remain unchanged. The projection enum itself has no Serde implementation. This adds one allocation for ready projections without a measured performance claim; the crate remains `publish = false`.

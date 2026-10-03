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

# Pass 01: Inventory and Documentation Findings

Audit date: 2026-04-21

## Scope
This pass mapped owned source surfaces and checked broad compliance triggers:
file size, required source-directory READMEs, generated/vendored boundaries,
and documentation completeness. Generated/build/vendor output was excluded from
file-size review unless it affected source-root policy.

## Standards Applied
- `CODING-STANDARDS.md`: 500-line file target, 250-line UI component review trigger, source-directory README requirement.
- `DOCUMENTATION-STANDARDS.md`: required README sections, banned placeholder language, host-facing and structured producer contracts.
- `ARCHITECTURE-PATTERNS.md`: package roles and directory structure traceability.

## Findings

### P01-F01: Large Files Exceed Decomposition Review Thresholds
Severity: High

Representative files over the 500-line target:
- `crates/pantograph-workflow-service/src/workflow.rs` - 7,848 lines.
- `crates/pantograph-embedded-runtime/src/lib.rs` - 6,640 lines.
- `crates/node-engine/src/core_executor.rs` - 4,248 lines.
- `crates/pantograph-embedded-runtime/src/task_executor.rs` - 2,861 lines.
- `crates/pantograph-embedded-runtime/src/model_dependencies.rs` - 2,611 lines.
- `crates/pantograph-runtime-registry/src/lib.rs` - 2,470 lines.
- `crates/inference/src/managed_runtime/operations.rs` - 2,420 lines.
- `crates/pantograph-rustler/src/lib.rs` - 2,340 lines.
- `src/components/WorkflowGraph.svelte` - 2,093 lines.
- `packages/svelte-graph/src/components/WorkflowGraph.svelte` - 1,225 lines.
- `src/components/nodes/workflow/DependencyEnvironmentNode.svelte` - 1,155 lines.

Impact:
- Several files are facades plus implementation plus tests or UI state machines.
- Review, ownership, and test targeting are harder than the standards allow.
- Multiple large files overlap with architecture and concurrency risks found in later passes.

### P01-F02: Required Source READMEs Are Missing
Severity: High

Missing README paths found under active source roots:
- `src/generated`
- `src-tauri/src/llm/commands/registry`
- `crates`
- `crates/pantograph-runtime-registry`
- `crates/pantograph-frontend-http-adapter`
- `crates/pantograph-uniffi`
- `crates/pantograph-rustler`
- `crates/pantograph-embedded-runtime`
- `crates/node-engine`
- `crates/pantograph-runtime-identity`
- `crates/pantograph-workflow-service`
- `crates/workflow-nodes`
- `crates/pantograph-workflow-service/tests`
- `crates/pantograph-workflow-service/examples`
- `crates/pantograph-workflow-service/src/workflow`
- `crates/inference/torch`
- `crates/inference/depth`
- `crates/inference/audio`
- `crates/inference/src/managed_runtime/llama_cpp_platform`
- `crates/inference/src/managed_runtime/managed_binaries`
- `crates/inference/src/managed_runtime/ollama_platform`

Notes:
- Many `src/` subdirectories do have README files, so this is not a total absence.
- The top-level `crates/` and crate root READMEs matter because this is a
  multi-package workspace with package-role standards.

### P01-F03: Existing READMEs Often Do Not Match Required Section Shape
Severity: Medium

Examples:
- `crates/pantograph-workflow-service/src/README.md` has useful ownership notes,
  but does not use the required `Problem`, `Constraints`, `Decision`,
  `Alternatives Rejected`, `Invariants`, `Revisit Triggers`, `Usage Examples`,
  `API Consumer Contract`, and `Structured Producer Contract` sections.
- `crates/pantograph-rustler/src/README.md` and
  `crates/pantograph-uniffi/src/README.md` document modes and dependencies but
  lack the full decision traceability structure expected for host-facing binding surfaces.

Impact:
- The current docs are valuable but inconsistent with the standards template.
- Host-facing surfaces need explicit lifecycle, error, retry, compatibility,
  and support-tier contracts.

### P01-F04: `src/generated` Contains a Nested Git Repository
Severity: Medium

`find` discovered `src/generated/.git` and nested Git metadata under a source
root. This may be intentional for generated component versioning, but it creates
source-root documentation and tooling ambiguity.

Impact:
- README enforcement and source scanning must explicitly account for this boundary.
- Source-root generated artifacts need a documented producer contract and cleanup policy.

### P01-F05: Decomposition Progress Exists but Is Incomplete
Severity: Medium

Prior docs show earlier compliance work:
- `docs/anti-pattern-remediation-tracker.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-standards-compliance-refactor-handoff.md`

The current inventory shows that earlier work improved several slices but did
not finish decomposition, source READMEs, or layered ownership cleanup.

## Additional Issues Outside Pure Standards Compliance
- Rust `cargo check` reports many dead-code warnings, especially in `src-tauri/src/workflow/*`, suggesting legacy or superseded Tauri-local workflow modules remain after backend service extraction.
- `src/generated/.git` should be classified as either supported generated-state infrastructure or removable accidental state.

## Pass 01 Remediation Themes
1. Add missing README files and normalize existing READMEs to required sections.
2. Split large facades and UI components around stable public facades.
3. Document generated-source ownership and nested Git behavior.
4. Convert dead-code warnings into either removal tasks or explicit disabled/experimental feature documentation.

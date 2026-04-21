# workflow-nodes

Built-in workflow node descriptors and task implementations for Pantograph.

## Purpose
This crate publishes Pantograph's built-in node inventory and node-specific
execution handlers. The boundary exists so node definitions can be linked into
`node-engine` through one Rust package instead of being scattered across app,
frontend, or binding code.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest and feature declarations for desktop and model-library integrations. |
| `src/` | Node descriptor modules, task implementations, setup helpers, and source-level README. |

## Problem
Workflow execution needs one authoritative inventory of built-in nodes, their
ports, categories, metadata, and task behavior. If descriptors and handlers are
duplicated by frontend code or adapters, saved workflows and runtime execution
can disagree about what a node means.

## Constraints
- Node metadata must stay aligned with executor behavior in `node-engine`.
- Optional model-library behavior must remain feature-gated.
- Host-specific execution should not leak into generic node descriptors.
- Disabled or experimental node behavior must not report successful output
  unless it really executed.

## Decision
Keep built-in node descriptors and implementations in this crate and register
them through `inventory`. Runtime hosts compose this crate with `node-engine`
and higher-level services, while UI code consumes projected metadata instead of
owning node truth.

## Alternatives Rejected
- Hard-code built-in node metadata in the frontend: rejected because workflow
  execution and saved graphs require backend-owned node contracts.
- Put all built-in node handlers inside `node-engine`: rejected because the
  engine should stay focused on execution orchestration and shared graph
  contracts.
- Make model-library integration unconditional: rejected because not every
  consumer needs Pumas-backed model discovery.

## Invariants
- Every exported descriptor must correspond to actual backend behavior or be
  clearly disabled/experimental.
- Descriptor category, port, and task metadata must match executor behavior.
- Optional features must not be required for base descriptor discovery unless
  documented.
- Runtime-specific dependencies stay behind feature gates.

## Revisit Triggers
- A second product wants a different built-in node catalog.
- Disabled tool execution is replaced by a real backend tool contract.
- Node registration moves from link-time inventory to generated descriptors.

## Dependencies
**Internal:** `node-engine` and optional `pumas-library`.

**External:** `graph-flow`, `tokio`, `async-trait`, `serde`, `serde_json`,
`reqwest`, `thiserror`, `log`, `uuid`, and `inventory`.

## Related ADRs
- `None identified as of 2026-04-21.`
- `Reason: This crate documents an existing built-in node inventory boundary.`
- `Revisit trigger: Node descriptors become generated schemas or are split by
  product/package.`

## Usage Examples
```rust
workflow_nodes::setup_extensions(&mut extensions).await;
```

## API Consumer Contract
- Inputs: node setup calls, task execution inputs, and optional model-library
  context supplied by runtime hosts.
- Outputs: task metadata, registered descriptors, node execution results, and
  typed node errors.
- Lifecycle: hosts call setup before executing workflows that need built-in
  nodes.
- Errors: task failures must be explicit and must not be masked as successful
  placeholder output.
- Versioning: descriptor changes that affect saved workflows require migration
  review.

## Structured Producer Contract
- Stable fields: node ids, categories, ports, labels, task metadata, and output
  shapes are consumed by workflow service, UI projections, and saved graphs.
- Defaults: optional metadata must have documented default behavior.
- Enums and labels: node categories and port data types carry execution
  semantics.
- Ordering: descriptor listing order is not a correctness contract.
- Compatibility: removing or renaming node ids can break saved workflows.
- Regeneration/migration: generated descriptor flows must update backend tests,
  frontend projections, and saved-workflow migrations together.

## Testing
```bash
cargo test -p workflow-nodes
```

## Notes
- Tool execution nodes currently fail explicitly until backend-owned tool
  runtime contracts exist.

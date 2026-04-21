# src-tauri/src/agent/tools

Desktop assistant tool implementation boundary.

## Purpose
This directory owns backend implementations for assistant tools that inspect or
modify local project files and Tailwind/CSS state.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Tool module exports and dispatch surface. |
| `error.rs` | Tool-specific error categories and conversions. |
| `validation.rs` | Shared path/input validation helpers. |
| `write_validation.rs` | Write-specific validation rules. |
| `write_versioning.rs` | Generated-component write history support that stores Git metadata in `.pantograph/generated-components.git/`. |
| `list.rs` | Directory/listing tool behavior. |
| `read.rs` | File read tool behavior. |
| `write.rs` | File write tool behavior. |
| `tailwind.rs` | Tailwind-specific inspection/helpers. |

## Problem
Assistant tools can read and write local files. They need explicit validation,
error mapping, and versioning boundaries so tool calls do not become unsafe
ad hoc filesystem operations.

## Constraints
- File paths must be validated before reads or writes.
- Write operations must preserve recoverability where versioning is supported.
- Tool results must be structured enough for agent consumers.
- Tool behavior must not bypass repository ownership rules.
- Generated-component write history must not recreate nested Git metadata under
  `src/generated/`.

## Decision
Keep assistant tools in a focused backend module with shared validation and
write-versioning helpers. Higher-level agent code dispatches tools through this
boundary instead of performing direct filesystem operations. Generated
component writes commit through the shared Tauri versioning helper so history
metadata stays outside the source tree.
Tool validation helpers should accept path slices at their internal boundaries
so generated-component validation does not require needless owned path values.

## Alternatives Rejected
- Let the LLM/agent layer manipulate files directly: rejected because path
  validation and write safety must be deterministic.
- Put filesystem tools in frontend code: rejected because local filesystem
  access belongs in the backend/Tauri layer.
- Store generated-component history under `src/generated/.git/`: rejected
  because source-root generated docs now rely on a normal tracked README and
  ignored runtime files.

## Invariants
- Validation runs before filesystem mutation.
- Tool errors preserve enough detail for agent recovery and user reporting.
- Write-versioning behavior stays coupled to write operations.
- Generated-component write versioning uses `.pantograph/generated-components.git/`
  with `src/generated/` as the work tree.
- Tool names and result payloads are compatibility contracts for the agent
  layer.
- Listing and validation helpers must remain deterministic after mechanical
  lint cleanup; expression rewrites cannot relax path or import validation.

## Revisit Triggers
- Tool execution becomes part of workflow graph runtime.
- Tools need a permission/sandbox profile system.
- Tool schemas become generated JSON Schema artifacts.
- Generated-component history moves into an application data directory or
  backend-owned non-Git store.

## Dependencies
**Internal:** agent types, validation helpers, local filesystem roots,
generated-component versioning commands, and Tailwind/design-system assets.

**External:** filesystem APIs, Git CLI, and parser/tooling dependencies used by
specific tools.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use crate::agent::tools::ToolError;
```

## API Consumer Contract
- Inputs: tool names, file paths, tool arguments, and optional write payloads.
- Outputs: structured tool results and tool errors.
- Lifecycle: tools execute per agent/tool request and do not own long-lived
  services.
- Errors: validation, read, write, and parse errors remain distinguishable.
- Versioning: tool names and result payloads must migrate with agent consumers.

## Structured Producer Contract
- Stable fields: tool result keys, error labels, path fields, and versioning
  metadata are machine-consumed by agent code.
- Defaults: path roots and write behavior defaults must be explicit.
- Enums and labels: tool names and error kinds carry behavior.
- Ordering: listing results should remain deterministic.
- Compatibility: payload changes affect assistant prompt/tool handling.
- Regeneration/migration: update tool dispatch, generated-component versioning,
  tests, and docs with tool schema or history storage changes.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml agent::tools
```

## Notes
- This boundary should inform M2 tool execution hardening.
- Generated-component versioning is shared with `src-tauri/src/llm/commands/version.rs`.

# docs

## Purpose
This directory holds Pantograph's durable design and operator-facing
documentation. It exists so architectural decisions, implementation references,
and runtime-operation guidance stay reviewable in-repo instead of being spread
across commit history or transient chat context.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `adr/` | Accepted architecture decision records and the ADR index used by plans and READMEs. |
| `logs/` | Checked-in investigation logs and probe outputs that support selected runtime and dependency decisions. |
| `headless-embedding-api-v1.md` | Reference contract for the headless embedding service surface. |
| `headless-native-bindings.md` | Product-facing native/bindings packaging and usage guidance. |
| `python-runtime-separation.md` | Design notes and rationale for keeping Python runtime execution out of process. |
| `runtime-registry-debug-and-recovery.md` | Operator/developer guide for runtime-registry inspection, reclaim, recovery, and Milestone 6 rollout posture. |
| `testing-and-release-strategy.md` | Repository policy for hybrid test placement, cross-layer acceptance, and release smoke strategy. |
| `toolchain-policy.md` | Pinned Rust, Node, npm, and Python versions plus update policy. |

## Problem
Pantograph spans workflow orchestration, runtime management, bindings, and
desktop-host integration. Without a documented home for accepted architecture
decisions and operational references, source-of-truth planning drifts and
boundary decisions become easy to lose.

## Constraints
- Documentation must describe the real codebase, not speculative architecture.
- ADR filenames and referenced docs must remain stable once plans and READMEs
  depend on them.
- Checked-in logs and historical notes must be clearly separated from accepted
  design decisions.

## Decision
Keep `docs/` as Pantograph's durable documentation root. ADRs live under
`docs/adr/`, historical or investigative log artifacts live under `docs/logs/`,
and longer-form design/operator references live at the top level when they are
stable enough to guide implementation or maintenance work.

## Alternatives Rejected
- Keep design context only in implementation plans.
  Rejected because plans are milestone-scoped and do not replace durable
  reference material.
- Spread operator and architecture notes across source READMEs only.
  Rejected because some cross-cutting guidance spans multiple source roots.

## Invariants
- Accepted ADRs are indexed in `docs/adr/README.md`.
- Historical logs do not silently become accepted architecture guidance.
- Top-level docs in this directory should remain stable references rather than
  scratch notes.

## Revisit Triggers
- Documentation volume or audience split grows enough to require sub-sections
  beyond `adr/` and `logs/`.
- A new class of persisted machine-consumed artifact is stored under `docs/`.
- Operator guidance begins to diverge from architecture or source READMEs.

## Dependencies
**Internal:** implementation plans, module READMEs, ADR references, and CI or
packaging docs that point here.

**External:** None.

## Related ADRs
- `None identified as of 2026-04-16.`
- `Reason: This directory indexes documentation surfaces rather than defining a single architecture boundary.`
- `Revisit trigger: docs ownership or ADR process changes materially.`

## Usage Examples
```markdown
- Architecture decision: `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Runtime investigation logs: `docs/logs/README.md`
```

## API Consumer Contract
- Human readers use this directory as the durable entry point for Pantograph
  architecture and operator/developer reference material.
- File paths under `docs/` are stable references for plans, READMEs, and
  reviews unless a superseding change updates those links in the same commit.
- New persistent documentation that guides implementation should be placed
  here or in a documented source-directory README, not in transient notes.

## Structured Producer Contract
- `docs/` is primarily human-consumed and does not itself define a machine API.
- `Reason: the directory stores narrative docs and linked log artifacts rather
  than a single schema-backed producer contract.`
- `Revisit trigger: a generated manifest, schema, or other machine-consumed
  docs artifact is added under this directory.`

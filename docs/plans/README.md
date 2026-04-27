# Plans

## Purpose

This directory contains Pantograph implementation plan families. It exists so
staged work, implementation gates, and future execution instructions stay under
one documentation artifact layout instead of being scattered across the
repository root.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `pantograph-execution-platform/` | Implemented execution-platform plan set covering attribution, node contracts, runtime observability, diagnostics ledger, composition, bindings, and reusable stage gates. |
| `run-centric-gui-workbench/` | Draft staged plan set for the scheduler-first GUI workbench, active-run navigation, workflow versioning, scheduler events, retention, Library/Pumas audit, and local-first Network page. |
| `diagnostics-run-history-projection/` | Focused plan for diagnostics run-history projections. |
| `scheduler-only-workflow-execution/` | Focused plan for scheduler-owned workflow execution behavior. |
| `workflow-duration-expectations/` | Focused plan for workflow timing expectation behavior. |
| `workflow-run-identity-redesign/` | Focused plan for workflow run identity redesign. |

## Problem

Pantograph planning spans backend architecture, frontend behavior, diagnostics,
scheduler policy, and binding surfaces. Without a local index, related plan
families are hard to discover and it is easy to miss the intended execution
entry point for broad staged work.

## Constraints

- Plan artifacts must follow the external coding standards under
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Broad work should live in a slugged subdirectory with its own README.
- Numbered files encode dependency order within a plan family.
- Historical or completed plans should not be silently rewritten as current
  implementation instructions.

## Decision

Use `docs/plans/` as the project-level implementation planning index. Broad
work gets a dedicated subdirectory. Narrow plans may remain as one-file
subdirectories when that is enough to preserve context.

## Alternatives Rejected

- Keep plan indexes only in individual subdirectories.
  Rejected because a reader still needs a way to discover which plan family is
  current or relevant.
- Move all plans back to the repository root.
  Rejected because root-level planning artifacts conflict with the documented
  docs artifact layout.

## Invariants

- New broad plan families must include a subdirectory README.
- Plans that drive implementation must link to their requirement inputs.
- Implementation status, deviations, and verification notes belong in the
  relevant plan family, not transient chat context.

## Revisit Triggers

- Plan volume grows enough to require additional grouping such as active,
  completed, and historical plans.
- A generated plan index becomes necessary.
- The documentation artifact layout changes in the external standards.

## Dependencies

**Internal:** `../requirements/`, `../adr/`, source-directory READMEs, and
stage-specific implementation reports.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

## Related ADRs

- `None identified as of 2026-04-27.`
- `Reason: This directory indexes implementation plans rather than accepting a
  runtime architecture decision by itself.`
- `Revisit trigger: plan organization starts affecting source ownership,
  release policy, or public API compatibility.`

## Usage Examples

Start broad GUI work from:

```text
docs/plans/run-centric-gui-workbench/README.md
```

Start execution-platform follow-up work from:

```text
docs/plans/pantograph-execution-platform/README.md
```

## API Consumer Contract

- This directory does not expose runtime APIs.
- Human implementers consume these plans as staged instructions and must verify
  each selected plan against current source code before editing.
- Plans may be superseded by newer ADRs or implemented status notes; readers
  should follow links in the relevant plan family.

## Structured Producer Contract

- Stable artifact category: Markdown implementation plans.
- Plan directory names are lowercase, hyphen-separated slugs.
- Numbered plan files encode local dependency order.
- These files are manually maintained and are not generated from schemas.

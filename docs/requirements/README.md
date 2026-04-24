# Requirements

## Purpose

This directory contains product and architecture requirements notes for
Pantograph feature areas that are still being shaped before implementation
planning.

## Contents

| File | Description |
| ---- | ----------- |
| `pantograph-node-system.md` | Requirements for backend-owned node contracts, port authoring, composition, runtime-managed observability, managed capabilities, and binding-facing node semantics. |
| `pantograph-client-sessions-buckets-model-license-diagnostics.md` | Requirements for durable client/session/bucket identity and persistent model/license usage diagnostics. |

## Problem

Root-level requirement notes are easy to confuse with package-manager
`requirements.txt` files and make planning artifacts harder to navigate. These
files need a documentation-specific home that separates product requirements
from source-code dependency declarations.

## Constraints

- These notes are requirements, not implementation plans.
- Requirement notes must stay backend-contract focused and avoid prescribing
  crate-by-crate implementation before the relevant plan exists.
- Dependency manifests such as Python `requirements.txt` files do not belong in
  this directory.

## Decision

Use `docs/requirements/` for human-authored Pantograph requirements notes. Keep
file names product-focused and omit the repository-root `REQUIREMENTS-` prefix
because the directory already supplies the artifact category.

## Alternatives Rejected

- Keep requirements at the repository root: rejected because it scatters
  planning artifacts and conflicts with the documentation artifact layout.
- Name the directory `requirements-files`: rejected because it reads like
  dependency manifests rather than product requirements.

## Invariants

- Files in this directory are Markdown requirements artifacts, not package
  dependency inputs.
- Requirements define expected behavior, constraints, terminology, and
  invariants before implementation sequencing.
- Implementation plans that consume these requirements must link back to the
  relevant requirement files.

## Revisit Triggers

- A requirement note grows into staged implementation work and should move or
  split into `docs/plans/`.
- A dependency-management file is accidentally added here.
- Multiple requirement families need subdirectories for ownership clarity.

## Dependencies

**Internal:** `docs/plans/` consumes these notes when turning requirements into
implementation plans.

**External:** None.

## Related ADRs

- `None identified as of 2026-04-23.`
- `Reason: This is documentation organization, not a binding architecture
  decision by itself.`
- `Revisit trigger: Requirements ownership starts affecting crate or API
  boundaries.`

## Usage Examples

Use these files as inputs when drafting or reviewing implementation plans:

```text
docs/requirements/pantograph-node-system.md
docs/requirements/pantograph-client-sessions-buckets-model-license-diagnostics.md
```

## API Consumer Contract

- None.
- Reason: This directory does not expose runtime APIs.
- Revisit trigger: Generated docs or external documentation tooling begins
  consuming requirement metadata as a stable machine-readable contract.

## Structured Producer Contract

- Stable artifact category: Markdown requirements notes.
- File names should be lowercase, hyphen-separated, and product-specific.
- These files are manually maintained and are not generated from schemas.
- Consumers should treat headings and prose as human documentation, not stable
  machine-readable schema fields.

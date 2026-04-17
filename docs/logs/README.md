# docs/logs

## Purpose
This directory stores checked-in investigation and probe logs that support
Pantograph runtime and dependency decisions. The boundary exists so raw
historical evidence can be retained without being confused for accepted design
documents or current operational instructions.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `pumas-runtime-probe-*.log` | Runtime probe outputs captured while validating Pumas-backed execution paths. |
| `pumas-rpc-trado-*.log` | RPC/install investigation logs captured during Trado runtime review work. |
| `pumas-seed-known-deps-*.log` | Dependency-seeding logs used while validating known Pumas dependency resolution. |

## Problem
Some Pantograph design and runtime decisions rely on concrete historical probe
output. Without a documented place for those logs, the repo either loses the
evidence entirely or scatters raw artifacts where they are mistaken for active
contracts or hand-maintained docs.

## Constraints
- Logs are historical evidence, not accepted architecture guidance.
- Log filenames must stay descriptive enough to trace the source investigation.
- Checked-in logs should remain append-only artifacts unless a cleanup change
  deliberately removes obsolete material.

## Decision
Keep raw investigation logs under `docs/logs/` and document them as historical
artifacts. Other docs and plans may reference these files when a design choice
needs evidence, but operator guidance and architecture ownership stay in ADRs,
plans, or source-directory READMEs.

## Alternatives Rejected
- Keep raw logs outside version control.
  Rejected because some implementation and review history depends on durable
  evidence.
- Mix logs into top-level `docs/` beside accepted guidance.
  Rejected because that blurs the line between evidence and source-of-truth
  documentation.

## Invariants
- Files in this directory are historical captures, not the active contract.
- Consumers should read the matching plan, ADR, or README for the current
  decision rather than treating a log as normative guidance.
- New logs should use descriptive filenames that encode topic and date.

## Revisit Triggers
- Log volume grows enough to require topic subdirectories or archival policy.
- A log file becomes the only source for an active operational instruction.
- A generated validation artifact with a stable schema is added here.

## Dependencies
**Internal:** implementation plans, ADRs, and review notes that cite these logs
as supporting evidence.

**External:** None.

## Related ADRs
- `None identified as of 2026-04-16.`
- `Reason: this directory stores evidence artifacts rather than a stable architecture boundary.`
- `Revisit trigger: log retention or validation policy becomes an architectural concern.`

## Usage Examples
```markdown
See `docs/logs/pumas-runtime-probe-2026-02-27.log` for the captured probe
output that informed the related runtime plan.
```

## API Consumer Contract
- Human readers and reviewers use these files as supporting evidence only.
- Plans or docs that cite a log should also point to the current accepted
  decision document so readers can distinguish evidence from policy.
- No runtime code path should parse these logs as an execution input.

## Structured Producer Contract
- Log filenames are the primary stable handle for human traceability.
- Log contents are intentionally raw and do not promise a stable schema.
- `Reason: these files are retained evidence artifacts rather than
  machine-consumed contracts.`
- `Revisit trigger: a validator, parser, or generated manifest begins to depend
  on files in this directory.`

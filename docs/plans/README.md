# Implementation Plans

Only current or recently accepted plan authorities live here:

| Plan | Status and purpose |
| --- | --- |
| [Current-standards remediation](current-standards-remediation/plan.md) | Planned portfolio coordinating the 2026-09-03 audit findings. |
| [Image-generation workflow](current-image-generation-graphs/plan.md) | Blocked product plan for a real editor-to-artifact image workflow. |
| [Documentation consolidation](documentation-consolidation/plan.md) | Accepted cleanup and migration record. |

Each planned effort uses:

```text
<plan>/
  plan.md              # current objective, decisions, status, and one next slice
  execution-ledger.md  # dated execution and evidence history
  issues.md            # discovered issues and dispositions
  reports/             # optional detailed evidence
```

Implementation starts from one explicit `plan.md` path and operation. Do not
infer an active plan from recency or directory order. Completed historical
plans are recoverable from Git history and are not retained as a second source
of current instructions.

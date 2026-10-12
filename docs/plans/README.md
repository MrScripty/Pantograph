# Implementation Plans

Only current or recently accepted plan authorities live here:

| Plan | Status and purpose |
| --- | --- |
| [CPU scheduler completion and fair progress](cpu-scheduler/plan.md) | User-authorized CPU scheduler policy rollout; independently reviewed opt-in stages, with GPU/split and remote work deferred. |
| [Domain architecture and multimodal workflows](domain-architecture-and-multimodal/plan.md) | Active authority for current-standards review, domain ownership and real text/image workflows; milestone evidence and current model routing live in the plan. |
| [Current-standards remediation](current-standards-remediation/plan.md) | Superseded; audit findings and unresolved claims transfer to the active plan. |
| [Image-generation workflow](current-image-generation-graphs/plan.md) | Superseded; the real editor-to-artifact objective transfers to the active mixed-workflow plan. |
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

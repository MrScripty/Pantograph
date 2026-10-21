# Implementation Plans

Only current or recently accepted plan authorities live here:

| Plan | Status and purpose |
| --- | --- |
| [CPU scheduler completion and fair progress](cpu-scheduler/plan.md) | User-authorized CPU scheduler policy rollout; independently reviewed opt-in stages, with GPU/split and remote work deferred. |
| [Domain architecture and multimodal workflows](domain-architecture-and-multimodal/plan.md) | Active authority for current-standards review, domain ownership and real text/image workflows; milestone evidence and current model routing live in the plan. |
| [CPU text integration](text-generation-readiness/plan.md) | Current-main readiness/terminal integration, failed-producer recovery and cold reopen; trained T3 blocked on verified licensed assets and sanctioned access. |
| [CPU text terminal outcomes](cpu-text-terminal-outcomes/plan.md) | Bounded public failure/cancellation repair retained in the text integration; historical exact-source qualification. |
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

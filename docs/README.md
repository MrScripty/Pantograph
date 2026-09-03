# Pantograph Documentation

Start with the smallest document that owns your question. Git history, rather
than an in-tree archive, preserves completed plans and superseded notes.

## Start Here

| Question | Current authority |
| --- | --- |
| What is Pantograph and how do I run it? | [Project README](../README.md) |
| How is the system divided and what is it transitioning toward? | [Architecture](../ARCHITECTURE.md) |
| How do I set up and verify a change? | [Development](development.md) |
| How do I embed Pantograph? | [Headless workflow integration](headless-workflow.md) |
| How do Python workers, runtime inspection, and recovery work? | [Runtime operations](runtime-operations.md) |
| What can currently be released? | [Release](release.md) |
| Which architecture decisions are accepted? | [ADR index](adr/README.md) |
| What standards gaps are currently known? | [Current standards audit](audits/2026-09-03-current-standards/README.md) |
| What work is currently authorized? | [Current remediation portfolio](plans/current-standards-remediation/plan.md) |

## Current Plans

- [Standards remediation portfolio](plans/current-standards-remediation/plan.md)
- [Image-generation workflow](plans/current-image-generation-graphs/plan.md)
- [Documentation consolidation](plans/documentation-consolidation/plan.md)

Plans own current objective, order, scope, and acceptance. Their
`execution-ledger.md` files own history; `issues.md` owns open dispositions.
See the [plans index](plans/README.md) for the artifact rules.

## Documentation Rules

- `README.md` and `ARCHITECTURE.md` are the repository entry points.
- ADRs own durable decisions that outlive an implementation effort.
- Guides describe current consumer or operator behavior and link exact schemas
  to source rather than copying large type inventories.
- Audits describe a dated baseline; plans own subsequent remediation.
- A source-directory README exists only when that boundary has a real consumer
  or operator. Directory inventories and fixed headings are not required.
- Superseded plans, implementation narration, probe logs, and old compliance
  reports belong in Git history, not a parallel archive tree.

When a guide and executable behavior disagree, treat the claim as unavailable,
inspect the owning source/contract, and update the canonical guide with the
implementation change.

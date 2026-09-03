# Plan: Documentation Consolidation

**Plan status:** `Accepted`

**Current phase:** Milestone 3 — Accepted

**Next slice:** `none`

**Acceptance status:** `satisfied`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`

**Related ADRs:** [ADR index](../../adr/README.md)

## Objective

Make Pantograph documentation smaller and easier to navigate by retaining only
current product guidance, accepted decisions, active work, and useful consumer
instructions. Historical implementation narration remains recoverable through
Git rather than competing with current authority in the working tree.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| DC-A01 | Root and `docs/README.md` route readers to one current authority for architecture, development, runtime operation, headless use, releases, audits, decisions, and active plans. | `focused` | `not-applicable` | `automated` | `satisfied` | Retained-link and authority scan |
| DC-A02 | Completed/historical plan trees, retired audits, stale implementation notes, probe logs, and README-per-directory artifacts are absent from the working tree and remain recoverable from Git history. | `focused` | `not-applicable` | `automated` | `satisfied` | 500-path deletion inventory and Git status |
| DC-A03 | The image-generation effort has one standards-shaped `plan.md`, ledger, and issues file with one current status and no competing recovery overlay. | `contract` | `not-applicable` | `automated` | `satisfied` | Current plan-structure checker passed |
| DC-A04 | Every retained local Markdown link resolves, accepted ADRs have unique discoverable identities, and no retained document claims that currently failing gates pass. | `focused` | `not-applicable` | `automated` | `satisfied` | Link, identity, stale-claim, and whitespace scans passed |
| DC-A05 | The two pre-existing Pumas proposal changes retain their exact pre-cleanup content. | `focused` | `not-applicable` | `automated` | `satisfied` | Pre/post SHA-256 comparison matched |

## Scope

### In Scope

- Root documentation entry points and obsolete root plans/reports.
- `docs/`, documentation-only indexes, accepted ADR filenames/links, and plan artifacts.
- Source-directory README files created for the retired universal documentation rule.
- Current-plan references affected by consolidation.

### Out Of Scope

- Product source, test, CI, hook, package, or launcher behavior.
- Changes to the content of the two pre-existing Pumas proposals.
- Implementing the standards audit remediation plans.
- Rewriting the substantive decisions in accepted ADRs.

## Constraints And Assumptions

- Git history is the archive for deleted historical narration; no parallel
  `archive/` tree is created.
- Consumer-facing binding/package READMEs remain where users encounter them.
- `ARCHITECTURE.md`, accepted ADRs, the current audit, and current remediation
  portfolio remain durable authorities.
- A retained guide describes current behavior or clearly labels target and
  unavailable behavior; it does not preserve stale success claims.
- The Pumas proposal files are user-owned and excluded from all cleanup edits.

## Binding Decisions

| Decision | Owner | Replaces |
| --- | --- | --- |
| `docs/README.md` is the concise documentation map. | Documentation root | Fixed-heading directory manifesto |
| `docs/development.md` owns toolchain and truthful verification entry points. | Development guide | Rust workspace, toolchain, and testing strategy prose |
| `docs/runtime-operations.md` owns Python process configuration and runtime-registry recovery guidance. | Runtime operations | Separate runtime separation and recovery notes |
| `docs/headless-workflow.md` owns headless integration direction and routes exact schemas to code/binding guides. | Headless consumer guide | Copied v1 schema, migration, implementation notes, and duplicate binding overview |
| `docs/release.md` distinguishes current release capability from target contract. | Release guide | Aspirational release policy presented as implemented |
| Git history owns completed plans, reports, retired proposals/audits, and chronological logs. | Repository history | In-tree historical document warehouses |

## Systemic Finding Audit

- Invariant family and canonical owner: each durable subject has one current documentation owner.
- Bounded population: root Markdown, `docs/**`, source-tree READMEs, retained binding/package READMEs, and their inbound links.
- Consumer dispositions: retain current consumer guidance, consolidate overlapping guidance, delete superseded history, or repair links to a retained authority.
- Deletion alternatives: an in-tree archive was rejected because it preserves the discovery burden without current authority value.
- Stopping condition: retained links resolve, current plan structure passes, stale success claims are absent, and excluded proposal blobs match their pre-cleanup values.

## Simplicity And Ownership Review

**Applicability:** `not-applicable`

**Reason:** This effort removes and consolidates documentation artifacts without introducing or changing a product Module, Interface, Seam, Adapter, composition root, or runtime mechanism.

## Milestones

### Milestone 0: Inventory And Classification

**Goal:** Bound retained and removed documentation classes and protect user-owned work.

**Allowed write set:** this plan directory only.

**Gate:** Inventory totals and protected proposal identities are recorded in the ledger.

**Status:** `Accepted`

### Milestone 1: Consolidate Current Guidance

**Goal:** Write concise current authorities and normalize the active image plan and ADR index.

**Allowed write set:** root Markdown excluding Pumas proposals; `docs/**/*.md`; `crates/README.md`; retained binding/package/script READMEs.

**Gate:** Replacement documents contain current routes and the image plan passes the current plan-structure checker.

**Status:** `Accepted`

### Milestone 2: Remove Superseded Documentation

**Goal:** Delete historical and duplicated documentation without creating an archive tree.

**Allowed write set:** classified obsolete Markdown/log files under the repository root, `docs/`, `crates/`, `src/`, `src-tauri/`, and `packages/svelte-graph/src/`.

**Gate:** Removed paths are recoverable as tracked Git deletions; protected proposals are unchanged.

**Status:** `Accepted`

### Milestone 3: Repair Discovery And Accept

**Goal:** Repair retained references and record the final smaller documentation inventory.

**Allowed write set:** retained Markdown and this plan directory.

**Gate:** Local-link, plan-structure, ADR-identity, stale-claim, whitespace, and diff checks pass.

**Status:** `Accepted`

## Blockers

- `none`

## Re-Plan Triggers

- A document classified obsolete is the only current authority for supported consumer behavior.
- Deletion would modify either protected Pumas proposal or a non-documentation artifact.
- A retained external consumer requires a removed migration or compatibility promise.
- Consolidation reveals contradictory current product behavior that source inspection cannot resolve.

## Final Acceptance

- Acceptance status: `satisfied`
- Deferred follow-ups: tooling replacement and product remediation remain in the current-standards portfolio.
- Final status: `Accepted`

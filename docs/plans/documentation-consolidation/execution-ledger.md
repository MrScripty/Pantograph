# Documentation Consolidation Execution Ledger

Current authority: [plan.md](plan.md)

## 2026-09-03 — Inventory

- Counted 542 Markdown files before cleanup.
- Identified roughly 90,000 lines in legacy/completed planning families.
- Identified about 160 source-directory READMEs created under the retired
  universal README rule.
- Protected the pre-existing changes to
  `PROPOSAL-pumas-artifact-load-target-resolution.md` and
  `PROPOSAL-pumas-library-fast-model-snapshot.md` from cleanup edits.

## 2026-09-03 — Consolidation And Removal

- Established concise current authorities for development, runtime operations,
  headless integration, releases, documentation discovery, and active plans.
- Consolidated the image-generation work into `plan.md`,
  `execution-ledger.md`, and `issues.md` and removed its competing recovery
  overlay and milestone fragments.
- Renamed the duplicate Rust-workspace ADR identity to ADR-017 and kept the ADR
  index unique from ADR-001 through ADR-017.
- With explicit repository-owner authorization, removed 500 historic,
  duplicated, or generated Markdown/log files: 484 tracked files recoverable
  through Git and 16 untracked probe logs archived temporarily at
  `/tmp/pantograph-obsolete-untracked-docs-2026-09-03.tar.gz`.
- Preserved the ignored, untracked `agent_plan.md` because it is user-owned and
  not recoverable through Git; it is not part of the documentation index.

## 2026-09-03 — Acceptance Evidence

- Final inventory: 68 Markdown files and 8,141 lines, down from 542 Markdown
  files before cleanup.
- Every retained local Markdown link resolves.
- The documentation consolidation plan, current image plan, remediation
  portfolio, and all five focused remediation plans pass the current
  plan-structure checker.
- ADR headings form one unique ADR-001 through ADR-017 sequence.
- Stale-success scanning found only explicit statements that strict Clippy does
  not pass; no retained guide claims repository-wide compliance.
- No retained Markdown file has trailing whitespace.
- `git diff --check` passes for the complete working-tree diff.
- Protected proposal SHA-256 values remain
  `97a0eede755ff519e789635abd84d3fd722862122f06b869ede6cb1c4d4a72f3`
  and `e5e8ab6f84393cf761ec5053d5b52a078dedaf90e06ae029612cd084813747c6`.

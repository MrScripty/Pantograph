# Pumas, Pantograph and public Chrema CPU composition

Plan status: `Verifying`. Operation: `verify` under the delegated instruction to
implement, test, commit, push and open a draft PR. Acceptance: `partial`.
Current phase: committed-candidate evidence capture. Next slice: prepare the
implementation commit, retain its complete CPU bundle, and publish a draft PR.

## Objective and scope

Provide one runnable workflow in which public Chrema records host component
assignments, Pantograph executes its public scheduler graph using a model
indexed by Pumas, and a later process inspects actual outputs and provenance.
Reuse the existing native CPU embedding probe. The committed untrained BERT
fixture performs numerical inference; this is fixture qualification and is not
trained-model usefulness or PR75's required real-text gate.

Base: Pantograph `2253cbe6fe8f1969b347c61fccba31859b13a4ba`.
Pumas consumed pin: `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`.
Public Chrema pin: `508c8b8cb724b49c655dab8bc4c8336190e516c7`.
Candidate Pumas main observed separately: `ef9eead9fe6595f5b3152ef8df19c5dace30190b`;
it is not adopted without compatibility qualification coordinated by parent.

## Binding decisions and owners

- Pumas owns index, package facts, execution descriptor and load target. This
  example uses its current pinned owner API; no upstream source is changed.
- Pantograph owns descriptors, dependency resolution/readiness, executable
  publication, scheduler dispatch, attribution, artifacts and diagnostics.
- Chrema owns assignments, events and completion enforcement. Its public
  `agent-platform/journal` package is built from a clean exact public checkout.
  Export the completed memory journal as a versioned inspection record; it is
  not a PostgreSQL journal or resumable workflow checkpoint.
- The composition owns child process deadlines, logs, local bundle identity,
  evidence retention and input validation. It refuses reused output roots and
  mismatched dependency revisions. No automatic model/package provisioning.
- A fresh native inspector opens the existing diagnostics/artifact stores,
  rebuilds projections, reads actual retained bodies, and never creates a model
  runtime. The JavaScript inspector validates bundle checksums and cross-links.
- No PR75/77 source is reused. PR74/76 are in the base; PR77 currently reports
  mergeable and its documentation write set is disjoint from this slice.

## Composed-design admission: applicable

1. One supported path: Chrema host assignment -> native Pumas/Pantograph CPU
   probe -> separate native retained-state inspection -> Chrema completion ->
   immutable exported inspection bundle.
2. Model/runtime/readiness and journal state stay with existing owners. The
   new host only coordinates processes and verifies their returned records.
3. Callers supply exact Chrema checkout, freshly built probe, and new output
   directory. The host knows the bounded fixture/probe contract, package entry
   point, subprocess deadline and bundle format; no scheduler internals.
4. A Chrema API change affects its pin and composition tests. A native probe
   output change affects its exporter/inspector and contract tests. Pumas
   selection changes require owner/API compatibility checks and native runs.
5. Stable values are model/run/artifact identities and serialized owner
   projections. Public package export resolution hides Chrema source layout.
6. Native execution and journal export can fail separately; failures retain
   logs and terminal journal outcomes. Inspection never resubmits inference.
7. Deleting the host removes cross-repository composition. Deleting the
   inspector reintroduces fresh-process provenance checks in every consumer.
   No generic executor framework or new runtime registry is warranted.
8. Necessary complexity is child lifecycle and retained-state identity; keep
   it in this example. Existing inference and persistence APIs remain intact.

## Milestone M1: runnable and retained composition (`Verifying`)

Write set: `examples/pumas-chrema-cpu-demo/**`,
`crates/pantograph-embedded-runtime/examples/cpu_embedding_workflow_probe.rs`,
`crates/pantograph-embedded-runtime/examples/cpu_embedding_workflow_probe/retained_inspection.rs`,
this plan directory, and `.github/workflows/cpu-composition-contract.yml`.
Preserve existing probe modes and public schemas. No root dependency update,
production scheduler policy, Chrema/private Nearwork source, tag or release.

| Claim | Criterion | Evidence/environment/mode | Status |
| --- | --- | --- | --- |
| C1 | Exact pins and clean Chrema package resolve; wrong pins fail closed | [Integration contracts](../../../examples/pumas-chrema-cpu-demo/composition.test.mjs), Linux CPU, automated | satisfied |
| C2 | Actual Pumas-indexed Candle CPU graph output matches independent reference | Native binary + Chrema composition, Linux CPU, automated; post-commit bundle next | pending |
| C3 | Fresh native process reads run/model provenance and retained output after producer exit | Real SQLite/artifact reopening + bundle inspector, Linux CPU, automated; post-commit bundle next | pending |
| C4 | Failure, timeout, corrupted evidence and reused output refuse success while preserving prior evidence | [Integration contracts](../../../examples/pumas-chrema-cpu-demo/composition.test.mjs), Linux CPU, automated | satisfied |

## Blockers and replan triggers

No implementation blocker currently. PR75 trained-text gate remains separately
blocked by model access and its saved Python dependency; do not report it passed.
If a qualified trained model/runtime is actually available, record a distinct
result. Replan if the fixture cannot execute on current pinned APIs, reopened
artifacts lack attribution, or substantial upstream changes become necessary.
Report minimal upstream requirements to parent; do not collide with owners.

## Isolation and publication

Task-owned branch `feat/pumas-pantograph-chrema-cpu-demo-2026-10-14` targets main.
Saved `work` at `ca9edde6850cd58ade0b7e534bb4f58e704c2c64` remains untouched.
Parent owns integration and compatible release checkpoints. Retain the branch
while its draft PR is under review; retire only after parent disposition and
verified commit reachability. No task-created worktree. No release tags.

Standards read from public Coding-Standards
`188beda1fa477d21d576c233dd8c7f4c4c267d23`: Core/Router, Planning/discovery,
Implementation, Verification/oracles, Commit, Development Proportionality,
Architecture, Dependencies, Rust, IPC and Persistence. No AGENTS.md or relevant
`.agents/skills` exists in the workspace/Pantograph/public Chrema checkouts.

[Ledger](execution-ledger.md) · [Issues](issues.md)
